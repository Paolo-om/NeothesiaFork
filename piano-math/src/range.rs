#![allow(dead_code)] // allows unused code to prevent warnings

// Define some constants for black keys
const KEY_CIS: u8 = 1;
const KEY_DIS: u8 = 3;
const KEY_FIS: u8 = 6;
const KEY_GIS: u8 = 8;
const KEY_AIS: u8 = 10;

// Define a struct for a piano key ID
#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub struct KeyId(u8);

impl KeyId {
    // Determine whether the key is black or white
    pub fn is_black(&self) -> bool {
        let key = self.0 % 12;
        key == KEY_CIS || key == KEY_DIS || key == KEY_FIS || key == KEY_GIS || key == KEY_AIS
    }
}

// Define a struct for a range of piano keys
#[derive(Debug, Clone)]
pub struct KeyboardRange {
    range: Range<u8>,

    keys: Vec<KeyId>,
    white_keys: Vec<KeyId>,
    black_keys: Vec<KeyId>,
}

impl KeyboardRange {
    // Create a new keyboard range
    pub fn new<R>(range: R) -> Self
    where
        R: RangeBounds<usize>,
    {
        let mut keys = Vec::new();
        let mut white_keys = Vec::new();
        let mut black_keys = Vec::new();

        // Convert the range bounds to u8 values
        let start = range.start_bound();
        let end = range.end_bound();

        let start = match start {
            std::ops::Bound::Included(id) => *id,
            std::ops::Bound::Excluded(id) => *id + 1,
            std::ops::Bound::Unbounded => 0,
        } as u8;

        let end = match end {
            std::ops::Bound::Included(id) => *id + 1,
            std::ops::Bound::Excluded(id) => *id,
            std::ops::Bound::Unbounded => 0,
        } as u8;

        let range = start..end;

        // Create the KeyId vectors for the range
        for id in range.clone().map(KeyId) {
            keys.push(id);

            if id.is_black() {
                black_keys.push(id);
            } else {
                white_keys.push(id);
            }
        }

        Self {
            range,

            keys,
            white_keys,
            black_keys,
        }
    }

    // Create a standard range for 88 keys
    pub fn standard_88_keys() -> Self {
        Self::new(21..=108)
    }
}

impl KeyboardRange {
    // Check whether an item is within the range
    pub fn contains(&self, item: u8) -> bool {
        self.range.contains(&item)
    }

    // Get the total count of keys in the range
    pub fn count(&self) -> usize {
        self.keys.len()
    }

    // Get the count of white keys in the range
    pub fn white_count(&self) -> usize {
        self.white_keys.len()
    }

    // Get the count of black keys in the range
    pub fn black_count(&self) -> usize {
        self.black_keys.len()
    }

    // Get an iterator for all keys in the range
    pub fn iter(&self) -> std::slice::Iter<KeyId> {
        self.keys.iter()
    }

    // Get an iterator for white keys in the range
    pub fn white_iter(&self) -> std::slice::Iter<KeyId> {
        self.white_keys.iter()
    }

    pub fn black_iter(&self) -> std::slice::Iter<KeyId> {
        self.black_keys.iter()
    }
}

impl Default for KeyboardRange {
    fn default() -> Self {
        Self::standard_88_keys()
    }
}

#[cfg(test)]
mod tests {}
