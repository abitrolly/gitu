use pest::Parser;
use pest_derive::Parser;
use std::fmt::Display;

#[derive(Debug, Clone)]
pub struct Diff {
    pub commit: Option<String>,
    pub deltas: Vec<Delta>,
}

impl Display for Diff {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        for delta in self.deltas.iter() {
            f.write_str(&delta.to_string())?;
        }

        Ok(())
    }
}

#[derive(Parser)]
#[grammar = "diff.pest"]
struct DiffParser;

impl Diff {
    pub fn parse(input: &str) -> Self {
        let mut commit = None;
        let mut deltas = vec![];

        for diff in DiffParser::parse(Rule::diffs, input).expect("Error parsing diff") {
            match diff.as_rule() {
                Rule::commit => commit = Some(diff.as_str().to_string()),
                Rule::diff => deltas.push(parse_diff(diff)),
                rule => panic!("No rule {:?}", rule),
            }
        }

        Self { commit, deltas }
    }
}

fn parse_diff(diff: pest::iterators::Pair<'_, Rule>) -> Delta {
    let mut old_file = None;
    let mut new_file = None;
    let mut file_header = None;
    let mut hunks = vec![];

    for diff_field in diff.into_inner() {
        match diff_field.as_rule() {
            Rule::diff_header => {
                file_header = Some(diff_field.as_str().to_string());
                let (old, new) = parse_diff_header(diff_field);
                old_file = Some(old);
                new_file = Some(new);
            }
            Rule::hunk => {
                let hunk = parse_hunk(
                    diff_field,
                    file_header.as_ref().unwrap(),
                    old_file.as_ref().unwrap(),
                    new_file.as_ref().unwrap(),
                );

                hunks.push(hunk);
            }
            rule => panic!("No rule {:?}", rule),
        }
    }

    Delta {
        file_header: file_header.unwrap(),
        old_file: old_file.unwrap(),
        new_file: new_file.unwrap(),
        hunks,
    }
}

fn parse_diff_header(diff_field: pest::iterators::Pair<'_, Rule>) -> (String, String) {
    let mut old_file = None;
    let mut new_file = None;

    for diff_header_field in diff_field.into_inner() {
        match diff_header_field.as_rule() {
            Rule::old_file => old_file = Some(diff_header_field.as_str().to_string()),
            Rule::new_file => new_file = Some(diff_header_field.as_str().to_string()),
            Rule::header_extra => {}
            rule => panic!("No rule {:?}", rule),
        }
    }

    (old_file.unwrap(), new_file.unwrap())
}

fn parse_hunk(
    diff_field: pest::iterators::Pair<'_, Rule>,
    file_header: &str,
    old_file: &str,
    new_file: &str,
) -> Hunk {
    let mut old_range = None;
    let mut new_range = None;
    let mut context = None;
    let mut body = None;

    for hunk_field in diff_field.into_inner() {
        match hunk_field.as_rule() {
            Rule::old_range => old_range = Some(parse_range(hunk_field)),
            Rule::new_range => new_range = Some(parse_range(hunk_field)),
            Rule::context => context = Some(hunk_field.as_str().to_string()),
            Rule::hunk_body => body = Some(hunk_field.as_str().to_string()),
            rule => panic!("No rule {:?}", rule),
        }
    }

    Hunk {
        file_header: file_header.to_string(),
        old_file: old_file.to_string(),
        new_file: new_file.to_string(),
        old_start: old_range.unwrap().0,
        old_lines: old_range.unwrap().1,
        new_start: new_range.unwrap().0,
        new_lines: new_range.unwrap().1,
        header_suffix: context.unwrap(),
        content: body.unwrap(),
    }
}

fn parse_range(hunk_field: pest::iterators::Pair<'_, Rule>) -> (u32, u32) {
    let mut start = None;
    let mut lines = None;

    for range_field in hunk_field.into_inner() {
        match range_field.as_rule() {
            Rule::start => {
                start = Some(
                    range_field
                        .as_str()
                        .parse()
                        .expect("Error parsing range start"),
                );
            }
            Rule::lines => {
                lines = Some(
                    range_field
                        .as_str()
                        .parse()
                        .expect("Error parsing range lines"),
                );
            }
            rule => panic!("No rule {:?}", rule),
        }
    }
    (
        start.expect("No range start"),
        lines.expect("No range lines"),
    )
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Delta {
    pub file_header: String,
    pub old_file: String,
    pub new_file: String,
    pub hunks: Vec<Hunk>,
}

impl Display for Delta {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.file_header)?;
        for hunk in self.hunks.iter() {
            f.write_str(&hunk.to_string())?;
        }

        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Hunk {
    pub file_header: String,
    pub old_file: String,
    pub new_file: String,
    pub old_start: u32,
    pub old_lines: u32,
    pub new_start: u32,
    pub new_lines: u32,
    header_suffix: String,
    pub content: String,
}

impl Hunk {
    pub fn display_header(&self) -> String {
        format!(
            "@@ -{},{} +{},{} @@",
            self.old_start, self.old_lines, self.new_start, self.new_lines
        )
    }

    pub fn header(&self) -> String {
        format!(
            "@@ -{},{} +{},{} @@{}",
            self.old_start, self.old_lines, self.new_start, self.new_lines, self.header_suffix
        )
    }

    pub fn format_patch(&self) -> String {
        format!("{}{}\n{}", &self.file_header, self.header(), &self.content)
    }
}

impl Display for Hunk {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.display_header())?;
        f.write_str(&self.content)?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::Diff;
    use pretty_assertions::assert_eq;

    #[test]
    fn parse_example() {
        let diff = Diff::parse(include_str!("example.patch"));
        assert_eq!(diff.deltas.len(), 2);
        assert_eq!(diff.deltas[0].hunks.len(), 2);
        assert_eq!(diff.deltas[1].hunks.len(), 2);
    }

    #[test]
    fn format_hunk_patch() {
        let diff = Diff::parse(include_str!("example.patch"));
        assert_eq!(
            diff.deltas[0].hunks[0].format_patch(),
            r#"diff --git a/src/diff.rs b/src/diff.rs
index 3757767..0aeba60 100644
--- a/src/diff.rs
+++ b/src/diff.rs
@@ -37,13 +37,12 @@ impl Diff {
             deltas: deltas_regex.captures_iter(&diff_str).map(|cap| {
                 let header = group_as_string(&cap, "header");
                 let hunk = group_as_string(&cap, "hunk");
+            dbg!("DELTA");
-                Delta {
-                    file_header: header.clone(),
-                    old_file: group_as_string(&cap, "old_file"),
-                    new_file: group_as_string(&cap, "new_file"),
-                    hunks: hunks_regex.captures_iter(&hunk)
+                let hunks = hunks_regex.captures_iter(&hunk)
                         .map(|hunk_cap| {
+            dbg!("HUNK");
+
                             Hunk {
                                 file_header: header.clone(),
                                 old_start: group_as_u32(&hunk_cap, "old_start"),
"#
        );
    }

    #[test]
    fn parse_example_empty_file() {
        let diff = Diff::parse(include_str!("example_empty_file.patch"));
        assert_eq!(diff.deltas.len(), 1);
        assert_eq!(diff.deltas[0].hunks.len(), 0);
    }
}
