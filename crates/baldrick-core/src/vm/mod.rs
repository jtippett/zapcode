use std::collections::{HashMap, HashSet};
use std::sync::Arc;

use indexmap::IndexMap;

use crate::compiler::instruction::{Constant, Instruction};
use crate::compiler::CompiledProgram;
use crate::error::{BaldrickError, Result};
use crate::sandbox::{ResourceLimits, ResourceTracker};
use crate::snapshot::BaldrickSnapshot;
use crate::value::{Closure, FunctionId, Value};

mod builtins;

/// The result of VM execution.
#[derive(Debug)]
pub enum VmState {
    Complete(Value),
    Suspended {
        function_name: String,
        args: Vec<Value>,
        snapshot: BaldrickSnapshot,
    },
}

/// A call frame in the VM stack.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
struct CallFrame {
    func_index: Option<usize>,
    ip: usize,
    locals: Vec<Value>,
    stack_base: usize,
}

/// The Baldrick VM.
pub struct Vm {
    program: CompiledProgram,
    stack: Vec<Value>,
    frames: Vec<CallFrame>,
    globals: HashMap<String, Value>,
    #[allow(dead_code)]
    stdout: String,
    limits: ResourceLimits,
    tracker: ResourceTracker,
    external_functions: HashSet<String>,
    try_stack: Vec<TryInfo>,
    /// The last object a property was accessed on — used for method dispatch.
    last_receiver: Option<Value>,
    /// The name of the last global loaded — used to identify known globals.
    last_global_name: Option<String>,
}

#[derive(Debug, Clone)]
struct TryInfo {
    catch_ip: usize,
    frame_depth: usize,
    stack_depth: usize,
}

impl Vm {
    fn new(
        program: CompiledProgram,
        limits: ResourceLimits,
        external_functions: HashSet<String>,
    ) -> Self {
        let mut globals = HashMap::new();

        // Register built-in globals
        builtins::register_globals(&mut globals);

        Self {
            program,
            stack: Vec::new(),
            frames: Vec::new(),
            globals,
            stdout: String::new(),
            limits,
            tracker: ResourceTracker::default(),
            external_functions,
            try_stack: Vec::new(),
            last_receiver: None,
            last_global_name: None,
        }
    }

    fn push(&mut self, value: Value) -> Result<()> {
        self.tracker.track_allocation(&self.limits)?;
        self.stack.push(value);
        Ok(())
    }

    fn pop(&mut self) -> Result<Value> {
        self.stack
            .pop()
            .ok_or_else(|| BaldrickError::RuntimeError("stack underflow".to_string()))
    }

    fn peek(&self) -> Result<&Value> {
        self.stack
            .last()
            .ok_or_else(|| BaldrickError::RuntimeError("stack underflow".to_string()))
    }

    fn current_frame(&self) -> &CallFrame {
        self.frames.last().expect("no active frame")
    }

    fn current_frame_mut(&mut self) -> &mut CallFrame {
        self.frames.last_mut().expect("no active frame")
    }

    #[allow(dead_code)]
    fn instructions(&self) -> &[Instruction] {
        match self.current_frame().func_index {
            Some(idx) => &self.program.functions[idx].instructions,
            None => &self.program.instructions,
        }
    }

    fn run(&mut self) -> Result<VmState> {
        self.tracker.start();

        // Set up top-level frame
        self.frames.push(CallFrame {
            func_index: None,
            ip: 0,
            locals: Vec::new(),
            stack_base: 0,
        });

        self.execute()
    }

    fn execute(&mut self) -> Result<VmState> {
        loop {
            // Resource checks
            self.tracker.check_time(&self.limits)?;

            let frame = self.frames.last().unwrap();
            let instructions = match frame.func_index {
                Some(idx) => &self.program.functions[idx].instructions,
                None => &self.program.instructions,
            };

            if frame.ip >= instructions.len() {
                // End of function/program
                if self.frames.len() <= 1 {
                    // Top-level: return last value on stack or undefined
                    let result = if self.stack.is_empty() {
                        Value::Undefined
                    } else {
                        self.stack.pop().unwrap_or(Value::Undefined)
                    };
                    return Ok(VmState::Complete(result));
                } else {
                    // Return from function
                    self.frames.pop();
                    self.tracker.pop_frame();
                    self.push(Value::Undefined)?;
                    continue;
                }
            }

            let instr = instructions[frame.ip].clone();
            let result = self.dispatch(instr);

            match result {
                Ok(Some(state)) => return Ok(state),
                Ok(None) => {}
                Err(err) => {
                    // Try to catch the error
                    if let Some(try_info) = self.try_stack.pop() {
                        // Unwind to catch block
                        while self.frames.len() > try_info.frame_depth {
                            self.frames.pop();
                            self.tracker.pop_frame();
                        }
                        self.stack.truncate(try_info.stack_depth);

                        // Push error value
                        let error_val = Value::String(Arc::from(err.to_string()));
                        self.push(error_val)?;

                        // Jump to catch
                        self.current_frame_mut().ip = try_info.catch_ip;
                    } else {
                        return Err(err);
                    }
                }
            }
        }
    }

    fn dispatch(&mut self, instr: Instruction) -> Result<Option<VmState>> {
        self.current_frame_mut().ip += 1;

        match instr {
            Instruction::Push(constant) => {
                let value = match constant {
                    Constant::Undefined => Value::Undefined,
                    Constant::Null => Value::Null,
                    Constant::Bool(b) => Value::Bool(b),
                    Constant::Int(n) => Value::Int(n),
                    Constant::Float(n) => Value::Float(n),
                    Constant::String(s) => Value::String(Arc::from(s.as_str())),
                };
                self.push(value)?;
            }
            Instruction::Pop => {
                self.pop()?;
            }
            Instruction::Dup => {
                let val = self.peek()?.clone();
                self.push(val)?;
            }
            Instruction::LoadLocal(idx) => {
                let frame = self.current_frame();
                let val = frame
                    .locals
                    .get(idx)
                    .cloned()
                    .unwrap_or(Value::Undefined);
                self.push(val)?;
            }
            Instruction::StoreLocal(idx) => {
                let val = self.pop()?;
                let frame = self.current_frame_mut();
                while frame.locals.len() <= idx {
                    frame.locals.push(Value::Undefined);
                }
                frame.locals[idx] = val;
            }
            Instruction::LoadGlobal(name) => {
                let val = self
                    .globals
                    .get(&name)
                    .cloned()
                    .unwrap_or(Value::Undefined);
                self.last_global_name = Some(name);
                self.push(val)?;
            }
            Instruction::StoreGlobal(name) => {
                let val = self.pop()?;
                self.globals.insert(name, val);
            }
            Instruction::DeclareLocal(_) => {
                let frame = self.current_frame_mut();
                frame.locals.push(Value::Undefined);
            }

            // Arithmetic
            Instruction::Add => {
                let right = self.pop()?;
                let left = self.pop()?;
                let result = match (&left, &right) {
                    (Value::Int(a), Value::Int(b)) => {
                        match a.checked_add(*b) {
                            Some(r) => Value::Int(r),
                            None => Value::Float(*a as f64 + *b as f64),
                        }
                    }
                    (Value::Float(a), Value::Float(b)) => Value::Float(a + b),
                    (Value::Int(a), Value::Float(b)) => Value::Float(*a as f64 + b),
                    (Value::Float(a), Value::Int(b)) => Value::Float(a + *b as f64),
                    (Value::String(a), _) => {
                        let mut s = a.to_string();
                        s.push_str(&right.to_js_string());
                        Value::String(Arc::from(s.as_str()))
                    }
                    (_, Value::String(b)) => {
                        let mut s = left.to_js_string();
                        s.push_str(b);
                        Value::String(Arc::from(s.as_str()))
                    }
                    _ => Value::Float(left.to_number() + right.to_number()),
                };
                self.push(result)?;
            }
            Instruction::Sub => {
                let right = self.pop()?;
                let left = self.pop()?;
                let result = match (&left, &right) {
                    (Value::Int(a), Value::Int(b)) => {
                        match a.checked_sub(*b) {
                            Some(r) => Value::Int(r),
                            None => Value::Float(*a as f64 - *b as f64),
                        }
                    }
                    _ => Value::Float(left.to_number() - right.to_number()),
                };
                self.push(result)?;
            }
            Instruction::Mul => {
                let right = self.pop()?;
                let left = self.pop()?;
                let result = match (&left, &right) {
                    (Value::Int(a), Value::Int(b)) => {
                        match a.checked_mul(*b) {
                            Some(r) => Value::Int(r),
                            None => Value::Float(*a as f64 * *b as f64),
                        }
                    }
                    _ => Value::Float(left.to_number() * right.to_number()),
                };
                self.push(result)?;
            }
            Instruction::Div => {
                let right = self.pop()?;
                let left = self.pop()?;
                let result = Value::Float(left.to_number() / right.to_number());
                self.push(result)?;
            }
            Instruction::Rem => {
                let right = self.pop()?;
                let left = self.pop()?;
                let result = match (&left, &right) {
                    (Value::Int(a), Value::Int(b)) if *b != 0 => Value::Int(a % b),
                    _ => Value::Float(left.to_number() % right.to_number()),
                };
                self.push(result)?;
            }
            Instruction::Pow => {
                let right = self.pop()?;
                let left = self.pop()?;
                let result = Value::Float(left.to_number().powf(right.to_number()));
                self.push(result)?;
            }
            Instruction::Neg => {
                let val = self.pop()?;
                let result = match val {
                    Value::Int(n) => Value::Int(-n),
                    _ => Value::Float(-val.to_number()),
                };
                self.push(result)?;
            }
            Instruction::BitNot => {
                let val = self.pop()?;
                let n = val.to_number() as i32;
                self.push(Value::Int(!n as i64))?;
            }
            Instruction::BitAnd => {
                let right = self.pop()?;
                let left = self.pop()?;
                let result = (left.to_number() as i32) & (right.to_number() as i32);
                self.push(Value::Int(result as i64))?;
            }
            Instruction::BitOr => {
                let right = self.pop()?;
                let left = self.pop()?;
                let result = (left.to_number() as i32) | (right.to_number() as i32);
                self.push(Value::Int(result as i64))?;
            }
            Instruction::BitXor => {
                let right = self.pop()?;
                let left = self.pop()?;
                let result = (left.to_number() as i32) ^ (right.to_number() as i32);
                self.push(Value::Int(result as i64))?;
            }
            Instruction::Shl => {
                let right = self.pop()?;
                let left = self.pop()?;
                let shift = (right.to_number() as u32) & 0x1f;
                let result = (left.to_number() as i32) << shift;
                self.push(Value::Int(result as i64))?;
            }
            Instruction::Shr => {
                let right = self.pop()?;
                let left = self.pop()?;
                let shift = (right.to_number() as u32) & 0x1f;
                let result = (left.to_number() as i32) >> shift;
                self.push(Value::Int(result as i64))?;
            }
            Instruction::Ushr => {
                let right = self.pop()?;
                let left = self.pop()?;
                let shift = (right.to_number() as u32) & 0x1f;
                let result = (left.to_number() as u32) >> shift;
                self.push(Value::Int(result as i64))?;
            }

            // Comparison
            Instruction::Eq | Instruction::StrictEq => {
                let right = self.pop()?;
                let left = self.pop()?;
                self.push(Value::Bool(left.strict_eq(&right)))?;
            }
            Instruction::Neq | Instruction::StrictNeq => {
                let right = self.pop()?;
                let left = self.pop()?;
                self.push(Value::Bool(!left.strict_eq(&right)))?;
            }
            Instruction::Lt => {
                let right = self.pop()?;
                let left = self.pop()?;
                self.push(Value::Bool(left.to_number() < right.to_number()))?;
            }
            Instruction::Lte => {
                let right = self.pop()?;
                let left = self.pop()?;
                self.push(Value::Bool(left.to_number() <= right.to_number()))?;
            }
            Instruction::Gt => {
                let right = self.pop()?;
                let left = self.pop()?;
                self.push(Value::Bool(left.to_number() > right.to_number()))?;
            }
            Instruction::Gte => {
                let right = self.pop()?;
                let left = self.pop()?;
                self.push(Value::Bool(left.to_number() >= right.to_number()))?;
            }

            // Logical
            Instruction::Not => {
                let val = self.pop()?;
                self.push(Value::Bool(!val.is_truthy()))?;
            }

            // Objects & Arrays
            Instruction::CreateArray(count) => {
                self.tracker.track_allocation(&self.limits)?;
                let mut arr = Vec::with_capacity(count);
                for _ in 0..count {
                    arr.push(self.pop()?);
                }
                arr.reverse();
                self.push(Value::Array(arr))?;
            }
            Instruction::CreateObject(count) => {
                self.tracker.track_allocation(&self.limits)?;
                let mut obj = IndexMap::new();
                // Pop key-value pairs (or spread values)
                let mut entries = Vec::new();
                for _ in 0..count {
                    let val = self.pop()?;
                    let key = self.pop()?;
                    entries.push((key, val));
                }
                entries.reverse();
                for (key, val) in entries {
                    match key {
                        Value::String(k) => {
                            obj.insert(k, val);
                        }
                        _ => {
                            let k: Arc<str> = Arc::from(key.to_js_string().as_str());
                            obj.insert(k, val);
                        }
                    }
                }
                self.push(Value::Object(obj))?;
            }
            Instruction::GetProperty(name) => {
                let obj = self.pop()?;
                let result = self.get_property(&obj, &name)?;
                // Store receiver for method calls
                if matches!(result, Value::BuiltinMethod { .. } | Value::Function(_)) {
                    self.last_receiver = Some(obj);
                }
                self.push(result)?;
            }
            Instruction::SetProperty(name) => {
                let value = self.pop()?;
                let obj_val = self.pop()?;
                match obj_val {
                    Value::Object(mut obj) => {
                        obj.insert(Arc::from(name.as_str()), value);
                        // Push modified object back so compile_store can store it
                        self.push(Value::Object(obj))?;
                    }
                    _ => {
                        return Err(BaldrickError::TypeError(format!(
                            "cannot set property '{}' on {}",
                            name,
                            obj_val.type_name()
                        )));
                    }
                }
            }
            Instruction::GetIndex => {
                let index = self.pop()?;
                let obj = self.pop()?;
                let result = match (&obj, &index) {
                    (Value::Array(arr), Value::Int(i)) => {
                        arr.get(*i as usize).cloned().unwrap_or(Value::Undefined)
                    }
                    (Value::Array(arr), Value::Float(f)) => {
                        arr.get(*f as usize).cloned().unwrap_or(Value::Undefined)
                    }
                    (Value::Object(map), Value::String(key)) => {
                        map.get(key.as_ref()).cloned().unwrap_or(Value::Undefined)
                    }
                    (Value::Object(map), _) => {
                        let key: Arc<str> = Arc::from(index.to_js_string().as_str());
                        map.get(key.as_ref()).cloned().unwrap_or(Value::Undefined)
                    }
                    (Value::String(s), Value::Int(i)) => {
                        s.chars()
                            .nth(*i as usize)
                            .map(|c| Value::String(Arc::from(c.to_string().as_str())))
                            .unwrap_or(Value::Undefined)
                    }
                    _ => Value::Undefined,
                };
                self.push(result)?;
            }
            Instruction::SetIndex => {
                let index = self.pop()?;
                let mut obj = self.pop()?;
                let value = self.pop()?;
                match &mut obj {
                    Value::Array(arr) => {
                        let idx = match &index {
                            Value::Int(i) => *i as usize,
                            Value::Float(f) => *f as usize,
                            _ => 0,
                        };
                        while arr.len() <= idx {
                            arr.push(Value::Undefined);
                        }
                        arr[idx] = value;
                    }
                    Value::Object(map) => {
                        let key: Arc<str> = Arc::from(index.to_js_string().as_str());
                        map.insert(key, value);
                    }
                    _ => {}
                }
                // Push modified object back so compile_store can store it to the variable
                self.push(obj)?;
            }
            Instruction::Spread => {
                // Handled contextually in CreateArray/CreateObject
            }
            Instruction::In => {
                let right = self.pop()?;
                let left = self.pop()?;
                let result = match &right {
                    Value::Object(map) => {
                        let key = left.to_js_string();
                        map.contains_key(key.as_str())
                    }
                    Value::Array(arr) => {
                        if let Value::Int(i) = left {
                            (i as usize) < arr.len()
                        } else {
                            false
                        }
                    }
                    _ => false,
                };
                self.push(Value::Bool(result))?;
            }
            Instruction::InstanceOf => {
                let _right = self.pop()?;
                let _left = self.pop()?;
                // No class support yet — always false
                self.push(Value::Bool(false))?;
            }

            // Functions
            Instruction::CreateClosure(func_idx) => {
                // Capture current scope for closure
                let mut captured = Vec::new();
                // Capture all locals from all active frames using local_names
                for frame in &self.frames {
                    let local_names = if let Some(fidx) = frame.func_index {
                        &self.program.functions[fidx].local_names
                    } else {
                        &self.program.local_names
                    };
                    for (i, val) in frame.locals.iter().enumerate() {
                        if let Some(name) = local_names.get(i) {
                            captured.push((name.clone(), val.clone()));
                        }
                    }
                }
                // Also capture all globals that are user-defined (not builtins)
                let builtins = ["console", "Math", "JSON", "Object", "Array"];
                for (name, val) in &self.globals {
                    if !builtins.contains(&name.as_str()) {
                        captured.push((name.clone(), val.clone()));
                    }
                }
                let closure = Closure {
                    func_id: FunctionId(func_idx),
                    captured,
                };
                self.push(Value::Function(closure))?;
            }
            Instruction::Call(arg_count) => {
                let mut args = Vec::with_capacity(arg_count);
                for _ in 0..arg_count {
                    args.push(self.pop()?);
                }
                args.reverse();

                let callee = self.pop()?;
                match callee {
                    Value::Function(closure) => {
                        let func_idx = closure.func_id.0;
                        self.tracker.push_frame();
                        self.tracker.check_stack(&self.limits)?;

                        let func = &self.program.functions[func_idx];

                        // Inject captured variables as globals
                        for (name, val) in &closure.captured {
                            if !self.globals.contains_key(name) {
                                self.globals.insert(name.clone(), val.clone());
                            }
                        }

                        // Set up locals with args
                        let mut locals = Vec::with_capacity(func.local_count);
                        for (i, param) in func.params.iter().enumerate() {
                            match param {
                                ParamPattern::Ident(_) => {
                                    locals.push(args.get(i).cloned().unwrap_or(Value::Undefined));
                                }
                                ParamPattern::Rest(_) => {
                                    let rest: Vec<Value> = args[i..].to_vec();
                                    locals.push(Value::Array(rest));
                                }
                                ParamPattern::DefaultValue { default: _, .. } => {
                                    let val = args.get(i).cloned().unwrap_or(Value::Undefined);
                                    if matches!(val, Value::Undefined) {
                                        locals.push(Value::Undefined);
                                    } else {
                                        locals.push(val);
                                    }
                                }
                                _ => {
                                    locals.push(args.get(i).cloned().unwrap_or(Value::Undefined));
                                }
                            }
                        }

                        self.frames.push(CallFrame {
                            func_index: Some(func_idx),
                            ip: 0,
                            locals,
                            stack_base: self.stack.len(),
                        });
                        self.last_receiver = None;
                    }
                    Value::BuiltinMethod { object_name, method_name } => {
                        let receiver = self.last_receiver.take();
                        let result = match object_name.as_ref() {
                            "__array__" => {
                                if let Some(Value::Array(arr)) = &receiver {
                                    builtins::call_builtin(
                                        &Value::Array(arr.clone()),
                                        &method_name,
                                        &args,
                                        &mut self.stdout,
                                    )?
                                } else {
                                    None
                                }
                            }
                            "__string__" => {
                                if let Some(Value::String(s)) = &receiver {
                                    builtins::call_builtin(
                                        &Value::String(s.clone()),
                                        &method_name,
                                        &args,
                                        &mut self.stdout,
                                    )?
                                } else {
                                    None
                                }
                            }
                            global_name => {
                                builtins::call_global_method(
                                    global_name,
                                    &method_name,
                                    &args,
                                    &mut self.stdout,
                                )?
                            }
                        };
                        match result {
                            Some(val) => self.push(val)?,
                            None => {
                                return Err(BaldrickError::TypeError(format!(
                                    "{}.{} is not a function",
                                    object_name, method_name
                                )));
                            }
                        }
                    }
                    _ => {
                        return Err(BaldrickError::TypeError(format!(
                            "{} is not a function",
                            callee.to_js_string()
                        )));
                    }
                }
            }
            Instruction::Return => {
                let return_val = self.pop().unwrap_or(Value::Undefined);

                if self.frames.len() <= 1 {
                    return Ok(Some(VmState::Complete(return_val)));
                }

                let frame = self.frames.pop().unwrap();
                self.tracker.pop_frame();
                self.stack.truncate(frame.stack_base);
                self.push(return_val)?;
            }
            Instruction::CallExternal(name, arg_count) => {
                if !self.external_functions.contains(&name) {
                    return Err(BaldrickError::UnknownExternalFunction(name));
                }
                let mut args = Vec::with_capacity(arg_count);
                for _ in 0..arg_count {
                    args.push(self.pop()?);
                }
                args.reverse();
                // Suspend execution
                let snapshot = BaldrickSnapshot::capture(self)?;
                return Ok(Some(VmState::Suspended {
                    function_name: name,
                    args,
                    snapshot,
                }));
            }

            // Control flow
            Instruction::Jump(target) => {
                self.current_frame_mut().ip = target;
            }
            Instruction::JumpIfFalse(target) => {
                let val = self.pop()?;
                if !val.is_truthy() {
                    self.current_frame_mut().ip = target;
                }
            }
            Instruction::JumpIfTrue(target) => {
                let val = self.pop()?;
                if val.is_truthy() {
                    self.current_frame_mut().ip = target;
                }
            }
            Instruction::JumpIfNullish(target) => {
                let val = self.peek()?;
                if matches!(val, Value::Null | Value::Undefined) {
                    self.current_frame_mut().ip = target;
                }
            }

            // Loops
            Instruction::SetupLoop => {}
            Instruction::Break | Instruction::Continue => {
                // These should have been compiled to jumps
            }

            // Iterators
            Instruction::GetIterator => {
                let val = self.pop()?;
                match val {
                    Value::Array(arr) => {
                        // Push an iterator object: [array, index]
                        let iter_obj = Value::Array(vec![
                            Value::Array(arr),
                            Value::Int(0),
                        ]);
                        self.push(iter_obj)?;
                    }
                    Value::String(s) => {
                        let chars: Vec<Value> = s
                            .chars()
                            .map(|c| Value::String(Arc::from(c.to_string().as_str())))
                            .collect();
                        let iter_obj = Value::Array(vec![
                            Value::Array(chars),
                            Value::Int(0),
                        ]);
                        self.push(iter_obj)?;
                    }
                    _ => {
                        return Err(BaldrickError::TypeError(format!(
                            "{} is not iterable",
                            val.type_name()
                        )));
                    }
                }
            }
            Instruction::IteratorNext => {
                let iter = self.pop()?;
                match iter {
                    Value::Array(ref items) if items.len() == 2 => {
                        let arr = match &items[0] {
                            Value::Array(a) => a,
                            _ => return Err(BaldrickError::RuntimeError("invalid iterator".into())),
                        };
                        let idx = match &items[1] {
                            Value::Int(i) => *i as usize,
                            _ => return Err(BaldrickError::RuntimeError("invalid iterator".into())),
                        };
                        if idx < arr.len() {
                            let value = arr[idx].clone();
                            // Update iterator
                            let new_iter = Value::Array(vec![
                                items[0].clone(),
                                Value::Int((idx + 1) as i64),
                            ]);
                            // Push updated iterator back, then the value
                            self.push(new_iter)?;
                            self.push(value)?;
                        } else {
                            // Done
                            self.push(iter)?;
                            self.push(Value::Undefined)?;
                        }
                    }
                    _ => {
                        return Err(BaldrickError::RuntimeError("invalid iterator state".into()));
                    }
                }
            }
            Instruction::IteratorDone => {
                // Check if the top value is the "done" sentinel
                let value = self.pop()?;
                let iter = self.peek()?;
                match iter {
                    Value::Array(items) if items.len() == 2 => {
                        let arr = match &items[0] {
                            Value::Array(a) => a,
                            _ => {
                                self.push(value)?;
                                self.push(Value::Bool(true))?;
                                return Ok(None);
                            }
                        };
                        let idx = match &items[1] {
                            Value::Int(i) => *i as usize,
                            _ => {
                                self.push(value)?;
                                self.push(Value::Bool(true))?;
                                return Ok(None);
                            }
                        };
                        let done = idx > arr.len();
                        if !done {
                            // Push value back for the binding
                            self.push(value)?;
                        }
                        self.push(Value::Bool(done))?;
                    }
                    _ => {
                        self.push(value)?;
                        self.push(Value::Bool(true))?;
                    }
                }
            }

            // Error handling
            Instruction::SetupTry(catch_ip, _) => {
                self.try_stack.push(TryInfo {
                    catch_ip,
                    frame_depth: self.frames.len(),
                    stack_depth: self.stack.len(),
                });
            }
            Instruction::Throw => {
                let val = self.pop()?;
                let msg = val.to_js_string();
                return Err(BaldrickError::RuntimeError(msg));
            }
            Instruction::EndTry => {
                self.try_stack.pop();
            }

            // Typeof
            Instruction::TypeOf => {
                let val = self.pop()?;
                let type_str = val.type_name();
                self.push(Value::String(Arc::from(type_str)))?;
            }

            // Void
            Instruction::Void => {
                self.pop()?;
                self.push(Value::Undefined)?;
            }

            // Update
            Instruction::Increment => {
                let val = self.pop()?;
                let result = match val {
                    Value::Int(n) => Value::Int(n + 1),
                    _ => Value::Float(val.to_number() + 1.0),
                };
                self.push(result)?;
            }
            Instruction::Decrement => {
                let val = self.pop()?;
                let result = match val {
                    Value::Int(n) => Value::Int(n - 1),
                    _ => Value::Float(val.to_number() - 1.0),
                };
                self.push(result)?;
            }

            // Template literals
            Instruction::ConcatStrings(count) => {
                let mut parts = Vec::with_capacity(count);
                for _ in 0..count {
                    parts.push(self.pop()?);
                }
                parts.reverse();
                let result: String = parts.iter().map(|v| v.to_js_string()).collect();
                self.push(Value::String(Arc::from(result.as_str())))?;
            }

            // Destructuring
            Instruction::DestructureObject(keys) => {
                let obj = self.pop()?;
                for key in keys {
                    let val = self.get_property(&obj, &key)?;
                    self.push(val)?;
                }
            }
            Instruction::DestructureArray(count) => {
                let arr = self.pop()?;
                match arr {
                    Value::Array(items) => {
                        for i in 0..count {
                            self.push(items.get(i).cloned().unwrap_or(Value::Undefined))?;
                        }
                    }
                    _ => {
                        for _ in 0..count {
                            self.push(Value::Undefined)?;
                        }
                    }
                }
            }

            Instruction::Nop => {}
        }

        Ok(None)
    }

    fn get_property(&self, obj: &Value, name: &str) -> Result<Value> {
        // Property access on null/undefined throws TypeError (like JS)
        if matches!(obj, Value::Null | Value::Undefined) {
            return Err(BaldrickError::TypeError(format!(
                "Cannot read properties of {} (reading '{}')",
                obj.to_js_string(),
                name
            )));
        }
        match obj {
            Value::Object(map) => {
                // Check if property exists as a real value on the object
                if let Some(val) = map.get(name) {
                    if !matches!(val, Value::Undefined) {
                        return Ok(val.clone());
                    }
                }
                // Check if this is a known global object — return builtin method handle
                if let Some(global_name) = &self.last_global_name {
                    let known_globals = ["console", "Math", "JSON", "Object", "Array"];
                    if known_globals.contains(&global_name.as_str()) {
                        return Ok(Value::BuiltinMethod {
                            object_name: Arc::from(global_name.as_str()),
                            method_name: Arc::from(name),
                        });
                    }
                }
                Ok(Value::Undefined)
            }
            Value::Array(arr) => {
                match name {
                    "length" => Ok(Value::Int(arr.len() as i64)),
                    _ if is_array_method(name) => Ok(Value::BuiltinMethod {
                        object_name: Arc::from("__array__"),
                        method_name: Arc::from(name),
                    }),
                    _ => {
                        if let Ok(idx) = name.parse::<usize>() {
                            Ok(arr.get(idx).cloned().unwrap_or(Value::Undefined))
                        } else {
                            Ok(Value::Undefined)
                        }
                    }
                }
            }
            Value::String(s) => {
                match name {
                    "length" => Ok(Value::Int(s.chars().count() as i64)),
                    _ if is_string_method(name) => Ok(Value::BuiltinMethod {
                        object_name: Arc::from("__string__"),
                        method_name: Arc::from(name),
                    }),
                    _ => Ok(Value::Undefined),
                }
            }
            _ => Ok(Value::Undefined),
        }
    }
}

// Re-export for the ParamPattern type used in function calls
use crate::parser::ir::ParamPattern;

fn is_array_method(name: &str) -> bool {
    matches!(
        name,
        "push" | "pop" | "shift" | "unshift" | "splice" | "slice" | "concat"
            | "join" | "reverse" | "sort" | "indexOf" | "lastIndexOf" | "includes"
            | "find" | "findIndex" | "map" | "filter" | "reduce" | "forEach"
            | "every" | "some" | "flat" | "flatMap" | "fill" | "at" | "entries"
            | "keys" | "values"
    )
}

fn is_string_method(name: &str) -> bool {
    matches!(
        name,
        "charAt" | "charCodeAt" | "indexOf" | "lastIndexOf" | "includes"
            | "startsWith" | "endsWith" | "slice" | "substring" | "substr"
            | "toUpperCase" | "toLowerCase" | "trim" | "trimStart" | "trimEnd"
            | "trimLeft" | "trimRight" | "padStart" | "padEnd" | "repeat"
            | "replace" | "replaceAll" | "split" | "concat" | "at"
            | "match" | "search" | "normalize"
    )
}

/// Main entry point: compile and run TypeScript code.
pub struct BaldrickRun {
    source: String,
    #[allow(dead_code)]
    inputs: Vec<String>,
    external_functions: Vec<String>,
    limits: ResourceLimits,
}

impl BaldrickRun {
    pub fn new(
        source: String,
        inputs: Vec<String>,
        external_functions: Vec<String>,
        limits: ResourceLimits,
    ) -> Result<Self> {
        Ok(Self {
            source,
            inputs,
            external_functions,
            limits,
        })
    }

    pub fn run(&self, input_values: Vec<(String, Value)>) -> Result<RunResult> {
        let program = crate::parser::parse(&self.source)?;
        let compiled = crate::compiler::compile(&program)?;
        let ext_set: HashSet<String> = self.external_functions.iter().cloned().collect();
        let mut vm = Vm::new(compiled, self.limits.clone(), ext_set);

        // Inject inputs as globals
        for (name, value) in input_values {
            vm.globals.insert(name, value);
        }

        let state = vm.run()?;
        Ok(RunResult {
            state,
            stdout: vm.stdout,
        })
    }

    pub fn run_simple(&self) -> Result<Value> {
        let result = self.run(Vec::new())?;
        match result.state {
            VmState::Complete(v) => Ok(v),
            VmState::Suspended { function_name, .. } => {
                Err(BaldrickError::RuntimeError(format!(
                    "execution suspended on external function '{}' — use run() instead",
                    function_name
                )))
            }
        }
    }
}

/// Result of running a Baldrick program.
pub struct RunResult {
    pub state: VmState,
    pub stdout: String,
}

/// Quick helper to evaluate a TypeScript expression.
pub fn eval_ts(source: &str) -> Result<Value> {
    let runner = BaldrickRun::new(
        source.to_string(),
        Vec::new(),
        Vec::new(),
        ResourceLimits::default(),
    )?;
    runner.run_simple()
}

/// Evaluate TypeScript and return both the value and stdout output.
pub fn eval_ts_with_output(source: &str) -> Result<(Value, String)> {
    let runner = BaldrickRun::new(
        source.to_string(),
        Vec::new(),
        Vec::new(),
        ResourceLimits::default(),
    )?;
    let result = runner.run(Vec::new())?;
    match result.state {
        VmState::Complete(v) => Ok((v, result.stdout)),
        VmState::Suspended { function_name, .. } => {
            Err(BaldrickError::RuntimeError(format!(
                "execution suspended on external function '{}'",
                function_name
            )))
        }
    }
}
