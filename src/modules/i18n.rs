use std::{
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
    locales: Vec<(String, Value)>,
}

impl I18n {
    pub fn new(locale: &str) -> Self {
        let mut i18n = Self {
            locale: Arc::new(Mutex::new(locale.to_string())),
            locales: Vec::new(),
        };
        i18n.load_locales();

        i18n
    }

    pub fn get(&self, key: &str) -> String {
        self.get_from_locale(&self.locale(), key)
    }

    pub fn get_from_locale(&self, locale: &str, key: &str) -> String {
        let mut value = String::from("KEY_NOT_FOUND");

        for (_, object) in self.locales.iter().filter(|(l, _)| l == locale) {
            if let Some(v) = object.get(key) {
                value = v.as_str().unwrap().to_string();
                break;
            }
        }
        value
    }

    pub fn locale(&self) -> String {
        let locale = self.locale.lock().unwrap();
        locale.to_string()
    }

    pub fn locales(&self) -> Vec<String> {
        self.locales
            .iter()
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
            self.locales.push((locale.to_string(), object));
        }
    }

    pub fn set_locale(&self, locale: impl Into<String>) {
        let locale = locale.into();

        if self.locales.iter().any(|(l, _)| *l == locale) {
            let mut curr_locale = self.locale.lock().unwrap();
            *curr_locale = locale.to_string();
        }
    }

    pub fn use_locale(&self, locale: &str) -> LocaleGuard {
        LocaleGuard::new(self, locale)
    }
}

impl Module for I18n {}
