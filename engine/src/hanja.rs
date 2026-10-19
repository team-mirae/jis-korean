//! The hanja dictionary (spec §7). Built by `dict/build_dict.py`; the engine only parses it.

use std::collections::HashMap;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Candidate {
    pub hanja: String,
    /// Meaning and reading for single characters (學: 배울 학). Often empty.
    pub comment: String,
}

#[derive(Debug, Default)]
pub struct Dictionary {
    entries: HashMap<String, Vec<Candidate>>,
    /// Longest reading, in syllables.
    longest: usize,
}

impl Dictionary {
    /// Parses `reading<TAB>hanja<TAB>comment` lines. Malformed lines are skipped.
    pub fn parse(tsv: &str) -> Dictionary {
        let mut dictionary = Dictionary::default();
        for line in tsv.lines() {
            let mut fields = line.split('\t');
            let (Some(reading), Some(hanja)) = (fields.next(), fields.next()) else { continue };
            if reading.is_empty() || hanja.is_empty() {
                continue;
            }
            let comment = fields.next().unwrap_or("").to_string();
            dictionary.longest = dictionary.longest.max(reading.chars().count());
            let candidates = dictionary.entries.entry(reading.to_string()).or_default();
            if candidates.iter().all(|c| c.hanja != hanja) {
                candidates.push(Candidate { hanja: hanja.to_string(), comment });
            }
        }
        dictionary
    }

    pub fn len(&self) -> usize {
        self.entries.len()
    }

    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    pub fn lookup(&self, reading: &str) -> &[Candidate] {
        self.entries.get(reading).map_or(&[], Vec::as_slice)
    }

    /// The longest trailing run of Hangul syllables in `text` that has candidates:
    /// "나는학교" → "학교". Returns the reading and its candidates.
    pub fn longest_suffix<'a>(&self, text: &'a str) -> Option<(&'a str, &[Candidate])> {
        let syllables: Vec<usize> = text
            .char_indices()
            .rev()
            .take_while(|(_, c)| is_syllable(*c))
            .map(|(i, _)| i)
            .collect();
        syllables.iter().take(self.longest).rev().find_map(|&start| {
            let reading = &text[start..];
            let candidates = self.lookup(reading);
            (!candidates.is_empty()).then_some((reading, candidates))
        })
    }
}

fn is_syllable(c: char) -> bool {
    ('\u{AC00}'..='\u{D7A3}').contains(&c)
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE: &str = "가\t可\t옳을 가\n가\t家\t집 가\n학교\t學校\t\n학\t學\t배울 학\n교\t校\t학교 교\n\
                          broken line\n가\t可\tduplicate\n";

    #[test]
    fn parses_and_keeps_order() {
        let d = Dictionary::parse(SAMPLE);
        assert_eq!(d.len(), 4);
        let hanja: Vec<_> = d.lookup("가").iter().map(|c| c.hanja.as_str()).collect();
        assert_eq!(hanja, ["可", "家"]);
        assert_eq!(d.lookup("가")[0].comment, "옳을 가");
        assert!(d.lookup("없음").is_empty());
    }

    #[test]
    fn longest_suffix_prefers_words() {
        let d = Dictionary::parse(SAMPLE);
        assert_eq!(d.longest_suffix("나는학교").map(|(r, _)| r), Some("학교"));
        assert_eq!(d.longest_suffix("나는교").map(|(r, _)| r), Some("교"));
        assert_eq!(d.longest_suffix("학교!"), None);
        assert_eq!(d.longest_suffix("ㄱ"), None);
        assert_eq!(d.longest_suffix(""), None);
    }
}
