#[derive(Clone)]
struct KeyMap {
    key: String,
    script: String,
}
struct KeyMapCollection {
    keymaps: Vec<KeyMap>,
}

impl KeyMapCollection {
    fn set(&mut self, arg_1: &str, arg_2: &str) {
        self.keymaps.push(KeyMap {
            key: arg_1.to_string(),
            script: arg_2.to_string(),
        });
    }

    fn new() -> Self {
        Self {
            keymaps: Vec::new(),
        }
    }

    fn get_keymaps(&self) -> Vec<KeyMap> {
        self.keymaps.clone()
    }
}

#[cfg(test)]
mod test {
    use crate::config::keymaps::KeyMapCollection;

    #[test]
    fn getting_key_maps_by_default_returns_empty() {
        let keymap = KeyMapCollection::new();
        let keymaps = keymap.get_keymaps();

        assert!(keymaps.is_empty());
    }

    #[test]
    fn getting_key_maps_after_adding_1_returns_that_key_map() {
        let keymap = &mut KeyMapCollection::new();
        keymap.set("y", "lua script");
        let keymaps = keymap.get_keymaps();

        assert!(!keymaps.is_empty());
        assert_eq!(keymaps.len(), 1);
        assert_eq!(keymaps[0].key, "y");
        assert_eq!(keymaps[0].script, "lua script");
    }
}
