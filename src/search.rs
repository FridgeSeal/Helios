use fuzzy_matcher::skim::SkimMatcherV2;
use fuzzy_matcher::FuzzyMatcher;

use crate::queries::{IndexData, PersistentQuery};

pub(crate) struct Searcher {
    matcher: SkimMatcherV2,
}

#[derive(Debug)]
pub(crate) struct MatchInformation {
    raw_text: String,
    score: i64,
    positions: Vec<[usize; 2]>,
}

pub(crate) struct TextSource {
    pub(crate) id: u32,
    pub(crate) name: Option<String>,
    data: String,
    // Feature idea -
}

impl TextSource {
    pub(crate) fn new(text: impl ToString, text_name: Option<String>) -> Self {
        Self {
            id: rand::random(),
            data: text.to_string(),
            name: text_name.into(),
        }
    }

    // Lazy loading from supported sources, etc
}

impl Searcher {
    pub fn new() -> Self {
        Self {
            matcher: SkimMatcherV2::default().ignore_case().use_cache(true),
        }
    }

    pub(crate) fn search_raw(&self, query: &str, text: &str) -> Option<MatchInformation> {
        self.matcher
            .fuzzy_indices(text, query)
            .map(|(score, positions)| {
                let indices = get_contiguous(&positions);
                MatchInformation {
                    raw_text: text.to_string(),
                    score,
                    positions: indices,
                }
            })
    }

    pub(crate) fn search(
        &self,
        query: &PersistentQuery,
        text_src: &TextSource,
    ) -> Option<IndexData> {
        self.search_raw(&query.query, &text_src.data)
            .filter(|match_info| match_info.score >= query.score_threshold)
            .map(|match_data| {
                println!("Running search on id: {} text: {}", query.id, query.query);
                IndexData {
                    source_query: query.id,
                    name: text_src.name.clone(),
                    key: rand::random::<u32>().into(),
                    document_id: text_src.id.into(),
                    match_indices: match_data.positions,
                    score: match_data.score,
                }
            })
    }
}

fn get_contiguous(v: &[usize]) -> Vec<[usize; 2]> {
    dbg!(&v);
    if v.len() < 2 {
        return Vec::new();
    }
    let mut arr: Vec<[usize; 2]> = Vec::with_capacity(v.len());

    let first = v.first().unwrap();
    let (mut minimum, mut maximum) = (first, first);
    for current in v.iter() {
        let difference = current - maximum;
        (minimum, maximum) = match difference {
            1 => {
                maximum = current;
                (minimum, maximum)
            }
            2.. => {
                arr.push([*minimum, *maximum]);
                minimum = current;
                maximum = current;
                (minimum, maximum)
            }
            _ => (minimum, maximum),
        };
    }
    arr.push([*minimum, *maximum]);
    arr.shrink_to_fit();
    dbg!(&arr);
    arr
}

#[cfg(test)]
mod search_tests {

    use super::*;
    use unicode_segmentation::UnicodeSegmentation;

    #[test]
    fn test_fuzzy_matching() {
        let src_text = "this is some text";
        let query = "text";
        let matcher = SkimMatcherV2::default();
        let res = matcher.fuzzy_match(src_text, query);
        dbg!(res);
        assert!(res.is_some())
    }

    #[test]
    fn test_search_large_text() {
        let matcher = SkimMatcherV2::default();
        let query = "prid n prejudice";
        let austen = include_str!("../data/pride_and_prejudice.txt");
        let frankenstein = include_str!("../data/frankenstein.txt");
        if let Some((score, indices)) = matcher.fuzzy_indices(austen, query) {
            println!("Score: {score}");
            let y = indices.get(0..=2).unwrap();
            dbg!(y);
            let x: String = austen
                .graphemes(true)
                .skip(y[0])
                .take(y[2] - y[0])
                .collect();
            dbg!(x);
        }
        let res2 = matcher.fuzzy_match(frankenstein, query);
        dbg!(res2);
    }

    #[test]
    fn test_get_contiguous_blocks() {
        let v = [1, 2, 3, 5, 6];
        let expected = get_contiguous(&v);
        assert_eq!(expected, [[1, 3], [5, 6]])
    }

    fn get_contiguous(v: &[usize]) -> Vec<[usize; 2]> {
        let mut arr: Vec<[usize; 2]> = Vec::with_capacity(v.len());

        let first = v.first().unwrap();
        let (mut minimum, mut maximum) = (first, first);
        for current in v.iter() {
            let difference = current - maximum;
            (minimum, maximum) = match difference {
                1 => {
                    maximum = current;
                    (minimum, maximum)
                }
                2.. => {
                    arr.push([*minimum, *maximum]);
                    minimum = current;
                    maximum = current;
                    (minimum, maximum)
                }
                0 | _ => (minimum, maximum),
            };
        }
        arr.push([*minimum, *maximum]);
        arr
    }
}
