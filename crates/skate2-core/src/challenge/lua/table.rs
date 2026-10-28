//! Lua 5.1 tables (`ltable.c`): an array part for keys `1..n` and an ordered
//! hash part. `len` is `luaH_getn`; `next` walks the array part, then the hash
//! part in insertion order.

use super::value::{Key, TableId, Value, array_index};
use std::collections::HashMap;

#[derive(Debug, Default)]
pub struct Table {
    array: Vec<Value>,
    entries: Vec<(Key, Value)>,
    index: HashMap<Key, usize>,
    live: usize,
    pub(crate) metatable: Option<TableId>,
}

/// Raw-set failures raised as Lua errors by the caller.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum KeyError {
    Nil,
    NaN,
}

impl KeyError {
    #[must_use]
    pub const fn message(self) -> &'static str {
        match self {
            Self::Nil => "table index is nil",
            Self::NaN => "table index is NaN",
        }
    }
}

impl Table {
    /// `luaH_new` with `OP_NEWTABLE`'s array/hash size hints.
    #[must_use]
    pub fn with_capacity(array: usize, hash: usize) -> Self {
        Self {
            array: vec![Value::Nil; array],
            entries: Vec::with_capacity(hash),
            index: HashMap::with_capacity(hash),
            live: 0,
            metatable: None,
        }
    }

    #[must_use]
    pub fn get(&self, key: &Value) -> Value {
        if let Value::Number(n) = key
            && let Some(i) = array_index(*n)
            && i <= self.array.len()
        {
            return self.array[i - 1].clone();
        }
        match Key::from_value(key) {
            Some(k) => self.get_hash(&k),
            None => Value::Nil,
        }
    }

    #[must_use]
    pub fn get_int(&self, i: usize) -> Value {
        if (1..=self.array.len()).contains(&i) {
            return self.array[i - 1].clone();
        }
        #[expect(clippy::cast_precision_loss, reason = "lua_Number key")]
        self.get_hash(&Key::Number((i as f32).to_bits()))
    }

    #[must_use]
    pub fn get_str(&self, key: &str) -> Value {
        self.get(&Value::string(key))
    }

    fn get_hash(&self, key: &Key) -> Value {
        self.index.get(key).map_or(Value::Nil, |&slot| self.entries[slot].1.clone())
    }

    /// `luaH_set` then assignment.
    pub fn set(&mut self, key: Value, value: Value) -> Result<(), KeyError> {
        if let Value::Number(n) = &key
            && let Some(i) = array_index(*n)
        {
            return {
                self.set_int(i, value);
                Ok(())
            };
        }
        let k = match &key {
            Value::Nil => return Err(KeyError::Nil),
            Value::Number(n) if n.is_nan() => return Err(KeyError::NaN),
            _ => Key::from_value(&key).expect("checked non-nil, non-NaN"),
        };
        self.set_hash(k, value);
        Ok(())
    }

    pub fn set_str(&mut self, key: &str, value: Value) {
        self.set_hash(Key::String(key.into()), value);
    }

    pub fn set_int(&mut self, i: usize, value: Value) {
        if (1..=self.array.len()).contains(&i) {
            self.array[i - 1] = value;
            return;
        }
        #[expect(clippy::cast_precision_loss, reason = "lua_Number key")]
        let key = Key::Number((i as f32).to_bits());
        if i == self.array.len() + 1 && !value.is_nil() {
            // A rehash would move the new border key and its successors here.
            self.remove_hash(&key);
            self.array.push(value);
            loop {
                #[expect(clippy::cast_precision_loss, reason = "lua_Number key")]
                let next = Key::Number(((self.array.len() + 1) as f32).to_bits());
                match self.index.get(&next).map(|&slot| self.entries[slot].1.clone()) {
                    Some(v) if !v.is_nil() => {
                        self.remove_hash(&next);
                        self.array.push(v);
                    }
                    _ => break,
                }
            }
            return;
        }
        self.set_hash(key, value);
    }

    fn set_hash(&mut self, key: Key, value: Value) {
        if let Some(&slot) = self.index.get(&key) {
            let entry = &mut self.entries[slot].1;
            match (entry.is_nil(), value.is_nil()) {
                (true, false) => self.live += 1,
                (false, true) => self.live -= 1,
                _ => {}
            }
            *entry = value;
            return;
        }
        if value.is_nil() {
            return;
        }
        if self.entries.len() >= 8 && self.live * 2 < self.entries.len() {
            self.compact();
        }
        self.index.insert(key.clone(), self.entries.len());
        self.entries.push((key, value));
        self.live += 1;
    }

    fn remove_hash(&mut self, key: &Key) {
        if let Some(&slot) = self.index.get(key) {
            if !self.entries[slot].1.is_nil() {
                self.live -= 1;
            }
            self.entries[slot].1 = Value::Nil;
        }
    }

    fn compact(&mut self) {
        self.entries.retain(|(_, v)| !v.is_nil());
        self.index.clear();
        for (slot, (k, _)) in self.entries.iter().enumerate() {
            self.index.insert(k.clone(), slot);
        }
    }

    /// `luaH_getn` (`0x829a11d0`) with `unbound_search`.
    #[must_use]
    pub fn len(&self) -> usize {
        let mut j = self.array.len();
        if j > 0 && self.array[j - 1].is_nil() {
            let mut i = 0;
            while j - i > 1 {
                let m = usize::midpoint(i, j);
                if self.array[m - 1].is_nil() {
                    j = m;
                } else {
                    i = m;
                }
            }
            return i;
        }
        if self.live == 0 {
            return j;
        }
        let mut i = j;
        j += 1;
        while !self.get_int(j).is_nil() {
            i = j;
            j *= 2;
            if j > i32::MAX as usize {
                let mut k = 1;
                while !self.get_int(k).is_nil() {
                    k += 1;
                }
                return k - 1;
            }
        }
        while j - i > 1 {
            let m = usize::midpoint(i, j);
            if self.get_int(m).is_nil() {
                j = m;
            } else {
                i = m;
            }
        }
        i
    }

    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// `luaH_next`: `None` ends the traversal; `Err` is an invalid key.
    pub fn next(&self, key: &Value) -> Result<Option<(Value, Value)>, ()> {
        let mut array_start = 0;
        let mut hash_start = 0;
        match key {
            Value::Nil => {}
            Value::Number(n) if array_index(*n).is_some_and(|i| i <= self.array.len()) => {
                array_start = array_index(*n).expect("checked");
            }
            _ => {
                let k = Key::from_value(key).ok_or(())?;
                let slot = *self.index.get(&k).ok_or(())?;
                array_start = self.array.len();
                hash_start = slot + 1;
            }
        }
        for i in array_start..self.array.len() {
            if !self.array[i].is_nil() {
                #[expect(clippy::cast_precision_loss, reason = "lua_Number key")]
                return Ok(Some((Value::Number((i + 1) as f32), self.array[i].clone())));
            }
        }
        for (k, v) in &self.entries[hash_start.min(self.entries.len())..] {
            if !v.is_nil() {
                return Ok(Some((k.to_value(), v.clone())));
            }
        }
        Ok(None)
    }

    /// Arena references for the collector's mark phase. Native traversetable
    /// (0x8299cf60) checks the value tag before marking. Host strings are Arc
    /// owned, so do not clone their keys just to discard them in the collector.
    pub(crate) fn for_each_reference(&self, mut f: impl FnMut(&Value)) {
        for v in &self.array {
            if matches!(v, Value::Table(_) | Value::Function(_) | Value::UserData(_)) {
                f(v);
            }
        }
        for (k, v) in &self.entries {
            if !v.is_nil() {
                match k {
                    Key::Table(t) => f(&Value::Table(*t)),
                    Key::Function(id) => f(&Value::Function(*id)),
                    Key::UserData(id) => f(&Value::UserData(*id)),
                    _ => {}
                }
                if matches!(v, Value::Table(_) | Value::Function(_) | Value::UserData(_)) {
                    f(v);
                }
            }
        }
    }

    #[cfg(test)]
    pub(crate) fn array_len(&self) -> usize {
        self.array.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn n(v: f32) -> Value {
        Value::Number(v)
    }

    #[test]
    fn sequences_grow_the_array_part() {
        let mut t = Table::default();
        for i in 1..=5 {
            t.set(n(i as f32), n(i as f32 * 10.0)).unwrap();
        }
        assert_eq!(t.len(), 5);
        assert_eq!(t.array_len(), 5);
        assert!(t.get(&n(3.0)).raw_equal(&n(30.0)));
        t.set(n(5.0), Value::Nil).unwrap();
        assert_eq!(t.len(), 4);
    }

    #[test]
    fn out_of_order_integer_keys_migrate() {
        let mut t = Table::default();
        t.set(n(3.0), n(3.0)).unwrap();
        t.set(n(2.0), n(2.0)).unwrap();
        assert_eq!(t.len(), 0);
        t.set(n(1.0), n(1.0)).unwrap();
        assert_eq!(t.array_len(), 3);
        assert_eq!(t.len(), 3);
    }

    #[test]
    fn next_walks_array_then_hash() {
        let mut t = Table::default();
        t.set(n(1.0), n(1.0)).unwrap();
        t.set_str("a", n(2.0));
        t.set_str("b", n(3.0));
        let mut key = Value::Nil;
        let mut seen = Vec::new();
        while let Some((k, _)) = t.next(&key).unwrap() {
            seen.push(format!("{k:?}"));
            key = k;
        }
        assert_eq!(seen.len(), 3);
        assert!(t.set(Value::Nil, n(1.0)).is_err());
        assert!(t.set(n(f32::NAN), n(1.0)).is_err());
        assert!(t.get(&n(-0.0)).is_nil());
        t.set(n(0.0), n(7.0)).unwrap();
        assert!(t.get(&n(-0.0)).raw_equal(&n(7.0)));
    }
}
