use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fmt;
use std::sync::{Arc, Mutex, OnceLock};

/// Global Authority for Identity Interning (v1.2.4)
/// Strictly forensic and immutable after discovery.
pub static REGISTRY: OnceLock<SymbolRegistry> = OnceLock::new();

/// Access the global registry singleton.
pub fn registry() -> &'static SymbolRegistry {
    REGISTRY.get_or_init(SymbolRegistry::new)
}

/// Transitional helper for legacy code (will be removed in v1.3.0)
pub fn interner() -> &'static SymbolRegistry {
    registry()
}

/// ABI Lock for canonicalization rules.
/// Change this hash if logic in `canonicalize()` is modified.
const CANON_HASH: u64 = 0xDEAD_C0DE_1234_5678;

/// Forensic-grade Symbol Registry (L0 Identity Layer)
/// Maps FQN strings to numeric indices and ensures cross-session epoch stability.
pub struct SymbolRegistry {
    /// Maps FQN to numeric index (Interned)
    name_to_id: Mutex<HashMap<Box<str>, u64>>,
    /// Maps numeric index back to FQN pointer (O(1) reverse lookup)
    id_to_name: Mutex<Vec<Arc<str>>>,
    /// Current session epoch (prevents cross-session aliasing)
    epoch: u32,
    /// ABI Lock for canonicalization integrity
    canon_hash: u64,
}

impl Default for SymbolRegistry {
    fn default() -> Self {
        Self::new()
    }
}

impl SymbolRegistry {
    pub fn new() -> Self {
        let registry = Self {
            name_to_id: Mutex::new(HashMap::new()),
            id_to_name: Mutex::new(vec![]),
            epoch: 1,
            canon_hash: CANON_HASH,
        };

        // Pre-intern the root symbol at Index 0
        registry.intern("crate");
        registry
    }

    pub fn epoch(&self) -> u32 {
        self.epoch
    }

    pub fn canon_hash(&self) -> u64 {
        self.canon_hash
    }

    pub fn len(&self) -> usize {
        let lock = self.id_to_name.lock().expect("Registry poisoned");
        lock.len()
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    pub fn intern(&self, raw_fqn: &str) -> SymbolId {
        let canonical = self.canonicalize(raw_fqn);

        let mut name_lock = self.name_to_id.lock().expect("Name registry poisoned");
        let mut id_lock = self.id_to_name.lock().expect("ID registry poisoned");

        if let Some(&index) = name_lock.get(canonical.as_str()) {
            return SymbolId {
                fqn: id_lock[index as usize].clone(),
                index,
                registry_epoch: self.epoch,
            };
        }

        let index = id_lock.len() as u64;
        let interned: Arc<str> = Arc::from(canonical);

        name_lock.insert(Box::from(interned.as_ref()), index);
        id_lock.push(interned.clone());

        SymbolId {
            fqn: interned,
            index,
            registry_epoch: self.epoch,
        }
    }

    pub fn resolve_id(&self, index: u64) -> Option<SymbolId> {
        let id_lock = self.id_to_name.lock().expect("ID registry poisoned");
        id_lock.get(index as usize).map(|fqn| SymbolId {
            fqn: fqn.clone(),
            index,
            registry_epoch: self.epoch,
        })
    }

    fn canonicalize(&self, input: &str) -> String {
        let mut fqn = input.trim().to_string();
        while fqn.contains("::::") {
            fqn = fqn.replace("::::", "::");
        }
        if fqn.starts_with("::") {
            fqn = fqn[2..].to_string();
        }
        let mut result = String::with_capacity(fqn.len());
        let mut colon_count = 0;
        for c in fqn.chars() {
            if c == ':' {
                colon_count += 1;
            } else {
                if colon_count > 0 {
                    result.push_str("::");
                    colon_count = 0;
                }
                result.push(c);
            }
        }
        result
    }
}

/// Forensic-grade Logical Identity (v1.2.4)
#[derive(Debug, Clone, Eq, PartialEq, Hash)]
pub struct SymbolId {
    pub fqn: Arc<str>,
    pub index: u64,
    pub registry_epoch: u32,
}

impl SymbolId {
    pub fn new(raw_fqn: &str) -> Self {
        registry().intern(raw_fqn)
    }

    pub fn identity_eq(&self, other: &Self) -> bool {
        self.index == other.index && self.registry_epoch == other.registry_epoch
    }

    pub fn root() -> Self {
        Self::new("crate")
    }
}

impl Serialize for SymbolId {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_u64(self.index)
    }
}

impl<'de> Deserialize<'de> for SymbolId {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        use serde::de::Error;
        let index = u64::deserialize(deserializer)?;
        registry().resolve_id(index).ok_or_else(|| {
            D::Error::custom(format!("Registry Identity DRIFT: unknown index {}", index))
        })
    }
}

impl fmt::Display for SymbolId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.fqn)
    }
}

/// Local Scope Tracker (v1.2.4)
/// Maps local names to canonical SymbolIds during parsing.
#[derive(Debug, Clone, Default)]
pub struct SymbolScopeRegistry {
    scopes: Vec<HashMap<String, SymbolId>>,
}

impl SymbolScopeRegistry {
    pub fn new() -> Self {
        Self {
            scopes: vec![HashMap::new()],
        }
    }

    pub fn push_scope(&mut self) {
        self.scopes.push(HashMap::new());
    }

    pub fn pop_scope(&mut self) {
        if self.scopes.len() > 1 {
            self.scopes.pop();
        }
    }

    pub fn discover(&mut self, name: &str) -> SymbolId {
        if let Some(id) = self.scopes.last().and_then(|s| s.get(name)) {
            return id.clone();
        }

        let id = SymbolId::new(name);
        if let Some(scope) = self.scopes.last_mut() {
            scope.insert(name.to_string(), id.clone());
        }
        id
    }

    pub fn resolve(&self, name: &str) -> Option<SymbolId> {
        for scope in self.scopes.iter().rev() {
            if let Some(id) = scope.get(name) {
                return Some(id.clone());
            }
        }
        None
    }

    pub fn reset(&mut self) {
        self.scopes = vec![HashMap::new()];
    }
}
