use std::{
    collections::HashMap,
    fs,
    sync::{Arc, Mutex},
};

use grammers_friendly::prelude::*;
use serde_json::Value;

const PATH: &str = "./assets/locale";

#[derive(Clone)]
pub struct LocaleGuard<'a> {
    i18n: &'a I18n,
    previous_locale: String,
}

impl<'a> LocaleGuard<'a> {
    pub fn new(i18n: &'a I18n, locale: &str) -> Self {
        let previous_locale = i18n.locale();
        i18n.set_locale(locale);

        LocaleGuard {
            i18n,
            previous_locale,
        }
    }
}

impl<'a> Drop for LocaleGuard<'a> {
    fn drop(&mut self) {
        self.i18n.set_locale(&self.previous_locale);
    }
}

#[derive(Clone)]
pub struct I18n {
    locale: Arc<Mutex<String>>,
    locales: HashMap<String, Value>,

    default_locale: String,
}

impl I18n {
    pub fn new(locale: &str) -> Self {
        let mut i18n = Self {
            locale: Arc::new(Mutex::new(locale.to_string())),
            locales: HashMap::new(),

            default_locale: locale.to_string(),
        };
        i18n.load_locales();

        i18n
    }

    pub fn get(&self, key: impl Into<String>) -> String {
        let key = key.into();

        self.get_from_locale(&self.locale(), &key)
    }

    pub fn get_from_locale(&self, locale: &str, key: &str) -> String {
        if let Some(object) = self.locales.get(locale) {
            match object.get(key) {
                Some(v) => v.as_str().unwrap().to_string(),
                None => {
                    if let Some(object) = self.locales.get(&self.default_locale) {
                        match object.get(key) {
                            Some(v) => v.as_str().unwrap().to_string(),
                            None => String::from("KEY_NOT_FOUND"),
                        }
                    } else {
                        String::from("LANGUAGE_NOT_FOUND")
                    }
                }
            }
        } else {
            String::from("LANGUAGE_NOT_FOUND")
        }
    }

    pub fn locale(&self) -> String {
        let locale = self.locale.lock().unwrap();
        locale.to_string()
    }

    pub fn locales(&self) -> Vec<String> {
        self.locales
            .clone()
            .into_iter()
            .map(|(locale, _)| locale.clone())
            .collect::<Vec<String>>()
    }

    pub fn load_locales(&mut self) {
        let locales = fs::read_dir(PATH)
            .unwrap()
            .map(|entry| {
                entry
                    .unwrap()
                    .file_name()
                    .to_str()
                    .unwrap()
                    .split_once(".")
                    .unwrap()
                    .0
                    .to_owned()
            })
            .collect::<Vec<String>>();

        for locale in locales.iter() {
            let path = format!("{}/{}.json", PATH, locale);
            let content = fs::read_to_string(&path).unwrap();
            let object: Value = serde_json::from_str(&content).unwrap();
            self.locales.insert(locale.to_string(), object);
        }
    }

    pub fn set_locale(&self, locale: impl Into<String>) {
        let locale = locale.into();

        if self.locales.get(&locale).is_some() {
            let mut curr_locale = self.locale.lock().unwrap();
            *curr_locale = locale;
        }
    }

    pub fn use_locale(&self, locale: &str) -> LocaleGuard {
        LocaleGuard::new(self, locale)
    }
}

impl Module for I18n {}
