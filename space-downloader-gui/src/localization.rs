use std::collections::HashMap;

use fluent_bundle::{FluentArgs, FluentBundle, FluentResource, FluentValue};
use unic_langid::LanguageIdentifier;

const EN_US: &str = include_str!("../locales/en-US/main.ftl");
const JA_JP: &str = include_str!("../locales/ja-JP/main.ftl");

macro_rules! lang {
    ($lang:literal) => {
        $lang
            .parse::<LanguageIdentifier>()
            .expect("invalid language")
    };
}

pub struct Localizer {
    current: LanguageIdentifier,
    bundles: HashMap<LanguageIdentifier, FluentBundle<FluentResource>>,
}

impl Localizer {
    pub fn new(default_language: &str) -> Self {
        let mut bundles = HashMap::new();
        for lang in Self::available_languages() {
            if let Some(bundle) = build_bundle(&lang) {
                bundles.insert(lang.clone(), bundle);
            }
        }

        let mut current = parse_language(default_language).unwrap_or_else(|| lang!("en-US"));
        if !bundles.contains_key(&current) {
            current = lang!("en-US");
        }

        Self { current, bundles }
    }

    pub fn available_languages() -> Vec<LanguageIdentifier> {
        vec![lang!("en-US"), lang!("ja-JP")]
    }

    #[allow(dead_code)]
    pub fn set_language(&mut self, language: &str) {
        if let Some(lang) = parse_language(language) {
            if self.bundles.contains_key(&lang) {
                self.current = lang;
            }
        }
    }

    #[allow(dead_code)]
    pub fn current_language(&self) -> &LanguageIdentifier {
        &self.current
    }

    pub fn text(&self, key: &str) -> String {
        self.format(key, None)
    }

    pub fn format(&self, key: &str, args: Option<&FluentArgs>) -> String {
        let bundle = match self.bundles.get(&self.current) {
            Some(bundle) => bundle,
            None => return key.to_string(),
        };

        if let Some(message) = bundle.get_message(key) {
            if let Some(pattern) = message.value() {
                let mut errors = Vec::new();
                let value = bundle.format_pattern(pattern, args, &mut errors);
                if errors.is_empty() {
                    return value.to_string();
                }
            }
        }

        key.to_string()
    }
}

fn parse_language(language: &str) -> Option<LanguageIdentifier> {
    language.parse().ok()
}

fn build_bundle(lang: &LanguageIdentifier) -> Option<FluentBundle<FluentResource>> {
    let source = match lang.to_string().as_str() {
        "ja-JP" => JA_JP,
        _ => EN_US,
    };
    let resource = FluentResource::try_new(source.to_owned()).ok()?;
    let mut bundle = FluentBundle::new(vec![lang.clone()]);
    bundle.add_resource(resource).ok()?;
    Some(bundle)
}

#[allow(dead_code)]
pub fn format_number(value: f64) -> FluentValue<'static> {
    FluentValue::from(value)
}
