pub const MAX_LEVEL: usize = 16;

#[derive(Clone, Debug, PartialEq)]
pub struct Prefix<'a> {
    pub text: &'a str,
    pub range: std::ops::Range<usize>,
}

#[cfg(test)]
impl<'a> From<&(&'a str, core::ops::Range<usize>)> for Prefix<'a> {
    fn from(value: &(&'a str, core::ops::Range<usize>)) -> Self {
        Self { text: value.0, range: value.1.clone() }
    }
}

struct Path<'a> {
    pub raw: &'a str,
    pub tokens: heapless::Vec<&'a str, MAX_LEVEL>,
}

impl<'a> From<&'a str> for Path<'a> {
    fn from(raw: &'a str) -> Self {
        Self { raw, tokens: raw.trim_matches('/').split('/').collect() }
    }
}

impl<'a> Path<'a> {
    fn pop(&mut self, n: usize) -> &'a str {
        if n == 0 {
            return "";
        }
        let remain = self.tokens.len() - n;
        let length = self.tokens[remain..].iter().fold(0, |acc, s| acc + s.len() + 1);
        let suffix;
        (self.raw, suffix) = self.raw.split_at(self.raw.len() - length);
        self.tokens.truncate(remain);
        suffix
    }

    fn match_tokens<'b>(&self, other: &Path<'b>) -> usize {
        self.tokens.iter().zip(other.tokens.iter()).take_while(|(a, b)| a == b).count()
    }

    fn num_level(&self) -> usize {
        self.tokens.len()
    }
}

impl<'a> Prefix<'a> {
    /// paths should be lexical ordered
    pub fn build<T: DoubleEndedIterator<Item = &'a str>>(paths: T) -> Vec<Prefix<'a>> {
        let mut retval = Vec::new();
        let mut iter = paths.rev().enumerate();
        let mut ref_path: Path = match iter.next() {
            Some((_, next)) => next.into(),
            None => return Vec::with_capacity(0),
        };
        let mut sum = 1;
        let mut ref_counts = heapless::Vec::<usize, MAX_LEVEL>::new();
        ref_counts.resize(ref_path.num_level(), 1).ok();
        for (index, raw_path) in iter {
            let path: Path = raw_path.into();
            let num_match = ref_path.match_tokens(&path);
            for i in 0..num_match {
                ref_counts[i] += 1;
            }
            let mut length = 1;
            for i in (num_match..ref_path.num_level()).rev() {
                if ref_counts[i] == 1 {
                    ref_path.pop(1);
                } else if i == 0 || ref_counts[i] < ref_counts[i - 1] {
                    let start = index - ref_counts[i];
                    retval.push(Prefix { text: ref_path.pop(length), range: start..index });
                    length = 1;
                } else if ref_counts[i] == ref_counts[i - 1] {
                    length += 1;
                } else {
                    ref_path.pop(1);
                }
            }
            ref_counts.truncate(ref_path.num_level());
            if path.num_level() > ref_path.num_level() {
                ref_counts.resize(path.num_level(), 1).ok();
                ref_path = path;
            }
            sum += 1;
        }
        let mut length = 1;
        for i in (0..ref_path.num_level()).rev() {
            if i == 0 || ref_counts[i] > 1 && ref_counts[i] < ref_counts[i - 1] {
                let range = sum - ref_counts[i]..sum;
                retval.push(Prefix { text: ref_path.pop(length), range });
                length = 1;
            } else if ref_counts[i] == ref_counts[i - 1] {
                length += 1;
            } else {
                ref_path.pop(1);
            }
        }
        retval.reverse();
        for i in 0..retval.len() {
            let range = &mut retval[i].range;
            *range = (sum - range.end)..(sum - range.start);
        }
        retval
    }
}

#[cfg(test)]
mod test {
    use super::Prefix;

    #[test]
    fn test_build_prefix() {
        let test_data = include_str!("../test/sample-data.yaml");
        let data = match serde_yaml::from_str(test_data).unwrap() {
            serde_yaml::Value::Mapping(map) => map,
            _ => panic!("Not a mapping"),
        };
        let mut paths: Vec<&str> = data.keys().map(|key| key.as_str().unwrap()).collect();
        paths.sort();
        let prefixes = Prefix::build(paths.into_iter());
        let expected = [
            ("/languages", 0..8),
            ("/C%2FC++", 0..3),
            ("/applications", 1..3),
            ("/go", 3..6),
            ("/applications", 4..6),
            ("/rust", 6..8),
        ];
        let expected: Vec<Prefix<'_>> = expected.iter().map(Into::into).collect();
        assert_eq!(expected, prefixes);
    }
}
