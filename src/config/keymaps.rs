use mlua::RegistryKey;
use std::sync::Arc;

use crate::prelude;

#[derive(Clone, Debug)]
pub enum Scope {
    Global,
    Pbi,
    Sprint,
}

pub struct KeyMap {
    pub key: String,
    pub func: Arc<RegistryKey>,
    pub description: Option<String>,
    pub scope: Scope,
    pub hidden: bool,
}

impl KeyMap {
    /// Execute the Lua function associated with this keymap.
    /// Returns the result string from the Lua function, or an error message.
    pub fn execute(&self) -> prelude::Result<String> {
        crate::lua::init::execute_keymap_action(self)
    }
}

pub struct KeyMapCollection {
    keymaps: Vec<KeyMap>,
}

impl Default for KeyMapCollection {
    fn default() -> Self {
        Self::new()
    }
}

impl KeyMapCollection {
    /// Register a keymap with a Lua function reference
    pub fn set(
        &mut self,
        key: &str,
        registry_key: RegistryKey,
        description: Option<&str>,
    ) -> prelude::Result<()> {
        if self.keymaps.iter().any(|k| k.key == key) {
            return Err(format!(r#"Key '{}' is already registered"#, key).into());
        }

        let scope = Scope::Global;
        self.keymaps.push(KeyMap {
            key: key.to_string(),
            func: Arc::new(registry_key),
            description: description.map(|s| s.to_string()),
            scope,
            hidden: false,
        });
        Ok(())
    }

    pub fn new() -> Self {
        Self {
            keymaps: Vec::new(),
        }
    }

    pub fn get_keymaps(&self) -> &[KeyMap] {
        &self.keymaps
    }

    #[allow(dead_code)]
    pub fn get_keymap(&self, key: &str) -> Option<&KeyMap> {
        self.keymaps.iter().find(|k| k.key == key)
    }
}

#[cfg(test)]
mod test {
    use mlua::Function;

    use crate::config::keymaps::{KeyMapCollection, Scope};

    #[test]
    fn getting_key_maps_by_default_returns_empty() {
        let keymap = KeyMapCollection::new();
        let keymaps = keymap.get_keymaps();

        assert!(keymaps.is_empty());
    }

    #[test]
    fn getting_key_maps_after_adding_1_returns_that_key_map() {
        let registry_key = lua_function();
        let keymap = &mut KeyMapCollection::new();
        keymap
            .set("y", registry_key, Some("some description about what this is"))
            .unwrap();
        let keymaps = keymap.get_keymaps();

        assert!(!keymaps.is_empty());
        assert_eq!(keymaps.len(), 1);
        assert_eq!(keymaps[0].key, "y");
        assert_eq!(
            keymaps[0].description,
            Some("some description about what this is".to_string())
        );
        assert!(matches!(keymaps[0].scope, Scope::Global));
    }

    #[test]
    fn getting_an_unregistered_key_returns_none() {
        let keymap = &mut KeyMapCollection::new();
        let keymap = keymap.get_keymap("y");

        assert!(keymap.is_none());
    }

    #[test]
    fn getting_a_registered_key_returns_the_keymap() {
        let registry_key = lua_function();

        let keymap = &mut KeyMapCollection::new();
        keymap
            .set("y", registry_key, Some("some description about what this is"))
            .unwrap();
        let keymap = keymap.get_keymap("y").unwrap();

        assert_eq!(keymap.key, "y");
        assert_eq!(keymap.description, Some("some description about what this is".to_string()));
        assert!(matches!(keymap.scope, Scope::Global));
    }

    #[test]
    fn setting_keymap_without_description() {
        let registry_key = lua_function();
        let keymap = &mut KeyMapCollection::new();
        keymap.set("y", registry_key, None).unwrap();
        let keymaps = keymap.get_keymaps();

        assert_eq!(keymaps.len(), 1);
        assert_eq!(keymaps[0].key, "y");
        assert_eq!(keymaps[0].description, None);
    }

    fn lua_function() -> mlua::RegistryKey {
        let lua = mlua::Lua::new();
        let my_func: Function = lua.create_function(|_, ()| Ok("hello")).unwrap();
        lua.create_registry_value(my_func).unwrap()
    }

    #[test]
    fn registering_the_same_key_twice_throws_an_error() {
        let registry_key = lua_function();
        let registry_key2 = lua_function();

        let keymap = &mut KeyMapCollection::new();
        keymap
            .set("y", registry_key, Some("some description about what this is"))
            .unwrap();

        match keymap
            .set("y", registry_key2, Some("some description about what this is"))
            .is_err()
        {
            true => {
                let keymaps = keymap.get_keymaps();
                assert!(!keymaps.is_empty());
                assert_eq!(keymaps.len(), 1);
            }
            false => {
                panic!("Expected an error when registering the same key twice");
            }
        }
    }
}
