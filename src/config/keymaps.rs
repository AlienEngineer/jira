use crate::prelude;

#[derive(Clone, Debug)]
#[allow(dead_code)]
pub enum Scope {
    Global,
    Pbi,
    Sprint,
}

#[derive(Clone, Debug)]
#[allow(dead_code)]
pub struct KeyMap {
    pub key: String,
    pub script: String,
    pub description: String,
    pub scope: Scope,
}
#[derive(Debug)]
pub struct KeyMapCollection {
    keymaps: Vec<KeyMap>,
}

impl KeyMapCollection {
    pub fn set(&mut self, key: &str, script: &str, description: &str) -> prelude::Result<()> {
        if self.keymaps.iter().find(|k| k.key == key).is_some() {
            return Err(format!(r#"Key '{}' is already registered"#, key).into());
        }

        let scope = Scope::Global; // For now, we only support global scope. This can be extended
                                   // in the future.
        self.keymaps.push(KeyMap {
            key: key.to_string(),
            script: script.to_string(),
            description: description.to_string(),
            scope,
        });
        Ok(())
    }

    pub fn new() -> Self {
        Self {
            keymaps: Vec::new(),
        }
    }

    pub fn get_keymaps(&self) -> Vec<KeyMap> {
        self.keymaps.clone()
    }

    #[allow(dead_code)]
    fn get_keymap(&self, key: &str) -> Option<KeyMap> {
        let keymap = self.keymaps.iter().find(|k| k.key == key);

        if keymap.is_some() {
            return keymap.cloned();
        }

        None
    }
}

#[cfg(test)]
mod test {
    use crate::config::keymaps::KeyMapCollection;
    use crate::config::keymaps::Scope;

    #[test]
    fn getting_key_maps_by_default_returns_empty() {
        let keymap = KeyMapCollection::new();
        let keymaps = keymap.get_keymaps();

        assert!(keymaps.is_empty());
    }

    #[test]
    fn getting_key_maps_after_adding_1_returns_that_key_map() {
        let keymap = &mut KeyMapCollection::new();
        keymap
            .set("y", "lua script", "some description about what this is")
            .unwrap();
        let keymaps = keymap.get_keymaps();

        assert!(!keymaps.is_empty());
        assert_eq!(keymaps.len(), 1);
        assert_eq!(keymaps[0].key, "y");
        assert_eq!(keymaps[0].script, "lua script");
        assert_eq!(
            keymaps[0].description,
            "some description about what this is"
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
        let keymap = &mut KeyMapCollection::new();
        keymap
            .set("y", "lua script", "some description about what this is")
            .unwrap();
        let keymap = keymap.get_keymap("y").unwrap();

        assert_eq!(keymap.key, "y");
        assert_eq!(keymap.script, "lua script");
        assert_eq!(keymap.description, "some description about what this is");
        assert!(matches!(keymap.scope, Scope::Global));
    }

    #[test]
    fn registering_the_same_key_twice_throws_an_error() {
        let keymap = &mut KeyMapCollection::new();
        keymap
            .set("y", "lua script", "some description about what this is")
            .unwrap();
        match keymap
            .set("y", "lua script", "some description about what this is")
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
