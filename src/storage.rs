use std::any::Any;
use std::marker::PhantomData;
use hashbrown::HashMap;
use parking_lot::Mutex;

pub trait Storage<Inner, S: State<Inner>> {
    fn fetch(&self, key: &str) -> Option<S>;

    fn save<IntoString: Into<String>>(&mut self, key: IntoString, value: S);
}

pub trait State<Body>: Clone {
    fn get_id(&self) -> String;

    fn get_expiration_time(&self) -> usize;
}

pub struct InMemoryStorage<A: Sized, S: State<A>> {
    store: HashMap<String, Mutex<S>>,
    _phantom_data: PhantomData<A>
}

impl<A: Sized, S: State<A>> InMemoryStorage<A, S> {
    pub fn new() -> Self {
        Self {
            store: HashMap::new(),
            _phantom_data: Default::default()
        }
    }
}

impl<A: Sized, S: State<A>> Storage<A, S> for InMemoryStorage<A, S> {
    fn fetch(&self, key: &str) -> Option<S> {
        if let Some(value) = self.store.get(key) {
            return Some(value.lock().clone());
        }

        None
    }

    fn save<IntoString: Into<String>>(&mut self, key: IntoString, value: S) {
        let key = key.into();

        if self.store.contains_key(&key) {
            let mut state = self.store.get(&key).unwrap().lock();
            *state = value;
        } else {
            self.store.insert(key, Mutex::new(value));
        }
    }
}

