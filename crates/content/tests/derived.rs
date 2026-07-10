//! Derived metadata: the pure AST word-counter and the display date formatter.

use content::{format_date, reading_minutes, ListItem, Node};

fn text(value: &str) -> Node {
    Node::Text {
        value: value.into(),
    }
}

fn prose(words: usize) -> String {
    vec!["word"; words].join(" ")
}

fn paragraph_of(words: usize) -> Node {
    Node::Paragraph {
        children: vec![text(&prose(words))],
    }
}

#[test]
fn read_time_rounds_up_at_200_wpm() {
    assert_eq!(reading_minutes(&[paragraph_of(1)]), 1);
    assert_eq!(reading_minutes(&[paragraph_of(200)]), 1);
    assert_eq!(reading_minutes(&[paragraph_of(201)]), 2);
    assert_eq!(reading_minutes(&[paragraph_of(800)]), 4);
}

// A wordless post still takes a moment; "0 min" would read as broken.
#[test]
fn read_time_floors_at_one_minute() {
    assert_eq!(reading_minutes(&[]), 1);
    assert_eq!(
        reading_minutes(&[Node::CodeBlock {
            lang: None,
            text: "let x = 1;".into(),
        }]),
        1
    );
}

#[test]
fn read_time_excludes_code_blocks() {
    let ast = [
        paragraph_of(100),
        Node::CodeBlock {
            lang: Some("rust".into()),
            text: prose(10_000),
        },
    ];
    assert_eq!(reading_minutes(&ast), 1);
}

// 200 words spread across every word-bearing shape count as one minute;
// one more word anywhere tips it to two — the counter reaches all of them.
#[test]
fn read_time_counts_words_in_nested_nodes() {
    let nested = |extra: usize| {
        vec![
            Node::Heading {
                level: 2,
                children: vec![text(&prose(10))],
            },
            Node::Paragraph {
                children: vec![
                    Node::Emphasis {
                        children: vec![text(&prose(40))],
                    },
                    Node::Strong {
                        children: vec![text(&prose(40))],
                    },
                    Node::Link {
                        url: "https://example.com".into(),
                        title: Some("ignored label".into()),
                        children: vec![text(&prose(30))],
                    },
                    Node::InlineCode { value: prose(20) },
                ],
            },
            Node::List {
                ordered: false,
                start: None,
                items: vec![ListItem {
                    children: vec![paragraph_of(30)],
                }],
            },
            Node::Blockquote {
                children: vec![paragraph_of(10)],
            },
            Node::Html {
                tag: "div".into(),
                attrs: Default::default(),
                children: vec![paragraph_of(10)],
            },
            Node::Component {
                name: "Callout".into(),
                props: Default::default(),
                children: vec![paragraph_of(10 + extra)],
            },
        ]
    };
    assert_eq!(reading_minutes(&nested(0)), 1);
    assert_eq!(reading_minutes(&nested(1)), 2);
}

// Alt text is described, not read in the prose flow.
#[test]
fn read_time_excludes_image_alt_text() {
    let ast = [
        paragraph_of(200),
        Node::Image {
            url: "/a.png".into(),
            alt: prose(500),
            title: None,
        },
    ];
    assert_eq!(reading_minutes(&ast), 1);
}

#[test]
fn dates_format_with_every_english_month_name() {
    let months = [
        "jan", "feb", "mar", "apr", "may", "jun", "jul", "aug", "sep", "oct", "nov", "dec",
    ];
    for (i, name) in months.iter().enumerate() {
        assert_eq!(
            format_date(&format!("2026-{:02}-15", i + 1)),
            format!("{name} 15, 2026")
        );
    }
}

#[test]
fn dates_keep_the_zero_padded_day() {
    assert_eq!(format_date("2026-07-04"), "jul 04, 2026");
    assert_eq!(format_date("2026-01-01"), "jan 01, 2026");
    assert_eq!(format_date("2026-12-31"), "dec 31, 2026");
}

// Display must never panic on stored data; anything off-shape passes through.
#[test]
fn malformed_dates_pass_through_unchanged() {
    for raw in [
        "someday",
        "",
        "2026-13-01",
        "2026-00-01",
        "2026-7-4",
        "2026-07",
    ] {
        assert_eq!(format_date(raw), raw);
    }
}
