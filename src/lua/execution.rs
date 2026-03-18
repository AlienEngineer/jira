use crate::{ioc::interface::Interface, prelude::Result};
use mlua::{Lua, Table};

pub trait ScriptIntegration: Interface {
    fn eval_script(&self, script: &str) -> Result<String>;
    fn exec_script(&self, script: &str) -> Result<()>;
    fn make_table(&self) -> Table;
    fn set_global(&self, key: &str, table: Table);
}

pub struct LuaIntegration {
    pub lua: Lua,
}

impl LuaIntegration {
    pub fn new() -> Self {
        let lua = Lua::new();
        Self { lua }
    }
}

impl Default for LuaIntegration {
    fn default() -> Self {
        Self::new()
    }
}

impl ScriptIntegration for LuaIntegration {
    fn eval_script(&self, script: &str) -> Result<String> {
        let lua = &self.lua;
        let result: String = lua.load(script).eval()?;
        Ok(result)
    }

    fn exec_script(&self, script: &str) -> Result<()> {
        let lua = &self.lua;
        if let Err(e) = lua.load(script).exec() {
            Err(format!("Failed to execute Lua script: {}", e).into())
        } else {
            Ok(())
        }
    }

    fn make_table(&self) -> Table {
        self.lua.create_table().unwrap()
    }

    fn set_global(&self, key: &str, table: Table) {
        self.lua.globals().set(key, table).unwrap();
    }
}
