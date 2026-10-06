use std::mem::take;

use pulldown_cmark::{CodeBlockKind, Event, Options, Parser, Tag, TagEnd};

use super::model::{Block, Face, Item, Link, Marker, Span, Style, Styled};

/// GitHub flavored markdown as blocks, with tables, task lists and
/// strikethrough.
#[cfg(test)]
pub(crate) fn parse(text: &str) -> Vec<Block> {
    parse_lines(text, false)
}

/// Like `parse`. With `keeps_line_breaks` a line break inside a paragraph
/// stays one, markdown itself reads it as a space.
pub(crate) fn parse_lines(text: &str, keeps_line_breaks: bool) -> Vec<Block> {
    let options = Options::ENABLE_TABLES | Options::ENABLE_STRIKETHROUGH | Options::ENABLE_TASKLISTS;
    let mut builder = Builder {
        soft_break: if keeps_line_breaks { "\n" } else { " " },
        ..Builder::default()
    };
    for event in Parser::new_ext(text, options) {
        builder.event(event);
    }
    builder.finish()
}

/// What blocks are collected into right now.
enum Frame {
    Quote(Vec<Block>),
    /// `next` is the number of the next item of a numbered list.
    List {
        next:  Option<u64>,
        items: Vec<Item>,
    },
    Item {
        marker: Marker,
        blocks: Vec<Block>,
    },
}

#[derive(Default)]
struct Table {
    head: Vec<Styled>,
    rows: Vec<Vec<Styled>>,
    row:  Vec<Styled>,
}

/// How deep the text is inside each kind of mark.
#[derive(Default)]
struct Depth {
    bold:   u32,
    italic: u32,
    strike: u32,
    link:   u32,
}

#[derive(Default)]
struct Builder {
    root:       Vec<Block>,
    frames:     Vec<Frame>,
    /// The text being collected. A tight list item has no paragraph
    /// around its text, so a text can open one by itself.
    inline:     Option<Styled>,
    depth:      Depth,
    /// Where the link being read starts in the text, and its address.
    link:       Option<(usize, String)>,
    heading:    Option<u8>,
    code:       Option<(String, String)>,
    table:      Option<Table>,
    /// What a line break inside a paragraph becomes.
    soft_break: &'static str,
}

impl Builder {
    fn event(&mut self, event: Event) {
        match event {
            Event::Start(tag) => self.start(tag),
            Event::End(tag) => self.end(tag),
            Event::Text(text) => match &mut self.code {
                Some((_, code)) => code.push_str(&text),
                None => self.text(&text, false),
            },
            Event::Code(text) => self.text(&text, true),
            Event::Html(text) | Event::InlineHtml(text) => self.text(&text, false),
            Event::SoftBreak => self.text(self.soft_break, false),
            Event::HardBreak => self.text("\n", false),
            Event::Rule => {
                self.flush();
                self.push(Block::Rule);
            }
            Event::TaskListMarker(done) => {
                if let Some(Frame::Item { marker, .. }) = self.frames.last_mut() {
                    *marker = Marker::Task(done);
                }
            }
            Event::FootnoteReference(_) | Event::InlineMath(_) | Event::DisplayMath(_) => {}
        }
    }

    fn start(&mut self, tag: Tag) {
        match tag {
            Tag::Paragraph => {
                self.flush();
                self.inline = Some(Styled::default());
            }
            Tag::Heading { level, .. } => {
                self.flush();
                self.heading = Some(level as u8);
                self.inline = Some(Styled::default());
            }
            Tag::BlockQuote(_) => {
                self.flush();
                self.frames.push(Frame::Quote(Vec::new()));
            }
            Tag::List(next) => {
                self.flush();
                self.frames.push(Frame::List {
                    next,
                    items: Vec::new(),
                });
            }
            Tag::Item => {
                let marker = match self.frames.last_mut() {
                    Some(Frame::List { next: Some(next), .. }) => {
                        let number = *next;
                        *next += 1;
                        Marker::Number(number)
                    }
                    _ => Marker::Bullet,
                };
                self.frames.push(Frame::Item {
                    marker,
                    blocks: Vec::new(),
                });
            }
            Tag::CodeBlock(kind) => {
                self.flush();
                let language = match kind {
                    CodeBlockKind::Fenced(info) => {
                        info.split_whitespace().next().unwrap_or_default().to_string()
                    }
                    CodeBlockKind::Indented => String::new(),
                };
                self.code = Some((language, String::new()));
            }
            Tag::Table(_) => {
                self.flush();
                self.table = Some(Table::default());
            }
            Tag::TableCell => self.inline = Some(Styled::default()),
            Tag::Strong => self.depth.bold += 1,
            Tag::Emphasis => self.depth.italic += 1,
            Tag::Strikethrough => self.depth.strike += 1,
            Tag::Link { dest_url, .. } => {
                self.depth.link += 1;
                let start = self.inline.as_ref().map_or(0, |text| text.text.len());
                self.link = Some((start, dest_url.to_string()));
            }
            _ => {}
        }
    }

    fn end(&mut self, tag: TagEnd) {
        match tag {
            TagEnd::Paragraph => self.flush(),
            TagEnd::Heading(_) => {
                let text = self.inline.take().unwrap_or_default();
                let level = self.heading.take().unwrap_or(1);
                self.push(Block::Heading { level, text });
            }
            TagEnd::BlockQuote(_) => {
                self.flush();
                if let Some(Frame::Quote(blocks)) = self.frames.pop() {
                    self.push(Block::Quote(blocks));
                }
            }
            TagEnd::List(_) => {
                if let Some(Frame::List { items, .. }) = self.frames.pop() {
                    self.push(Block::List(items));
                }
            }
            TagEnd::Item => {
                self.flush();
                if let Some(Frame::Item { marker, blocks }) = self.frames.pop()
                    && let Some(Frame::List { items, .. }) = self.frames.last_mut()
                {
                    items.push(Item { marker, blocks });
                }
            }
            TagEnd::CodeBlock => {
                if let Some((language, text)) = self.code.take() {
                    let text = text.trim_end_matches('\n').to_string();
                    self.push(Block::Code { language, text });
                }
            }
            TagEnd::TableCell => {
                let cell = self.inline.take().unwrap_or_default();
                if let Some(table) = &mut self.table {
                    table.row.push(cell);
                }
            }
            TagEnd::TableHead => {
                if let Some(table) = &mut self.table {
                    table.head = take(&mut table.row);
                }
            }
            TagEnd::TableRow => {
                if let Some(table) = &mut self.table {
                    let row = take(&mut table.row);
                    table.rows.push(row);
                }
            }
            TagEnd::Table => {
                if let Some(table) = self.table.take() {
                    self.push(Block::Table {
                        head: table.head,
                        rows: table.rows,
                    });
                }
            }
            TagEnd::Strong => self.depth.bold = self.depth.bold.saturating_sub(1),
            TagEnd::Emphasis => self.depth.italic = self.depth.italic.saturating_sub(1),
            TagEnd::Strikethrough => self.depth.strike = self.depth.strike.saturating_sub(1),
            TagEnd::Link => {
                self.depth.link = self.depth.link.saturating_sub(1);
                if let Some((start, url)) = self.link.take()
                    && let Some(text) = &mut self.inline
                    && start < text.text.len()
                {
                    text.links.push(Link {
                        range: start..text.text.len(),
                        url,
                    });
                }
            }
            _ => {}
        }
    }

    fn text(&mut self, text: &str, code: bool) {
        let style = Style {
            face: Face::new(self.depth.bold > 0, self.depth.italic > 0),
            code,
            strike: self.depth.strike > 0,
            link: self.depth.link > 0,
        };
        let styled = self.inline.get_or_insert_with(Styled::default);
        let start = styled.text.len();
        styled.text.push_str(text);
        let end = styled.text.len();
        if style == Style::default() {
            return;
        }
        match styled.spans.last_mut() {
            Some(last) if last.range.end == start && last.style == style => last.range.end = end,
            _ => styled.spans.push(Span {
                range: start..end,
                style,
            }),
        }
    }

    /// Ends the text being collected as a paragraph.
    fn flush(&mut self) {
        if let Some(text) = self.inline.take()
            && !text.text.trim().is_empty()
        {
            self.push(Block::Paragraph(text));
        }
    }

    fn push(&mut self, block: Block) {
        match self.frames.last_mut() {
            Some(Frame::Quote(blocks) | Frame::Item { blocks, .. }) => blocks.push(block),
            // The parser puts nothing between a list and its items.
            Some(Frame::List { .. }) => {}
            None => self.root.push(block),
        }
    }

    fn finish(mut self) -> Vec<Block> {
        self.flush();
        self.root
    }
}

#[cfg(test)]
mod tests {
    use super::{parse, parse_lines};
    use crate::ui::views::complex::markdown::model::{Block, Face, Marker, Style};

    #[test]
    fn inline_marks_become_spans_and_leave_the_text() {
        let blocks = parse("a **bold** and `code` and ~~gone~~ and [link](https://x.y)");
        let [Block::Paragraph(text)] = &blocks[..] else {
            panic!("one paragraph expected, got {blocks:?}");
        };
        assert_eq!(text.text, "a bold and code and gone and link");
        let styles: Vec<(&str, Style)> = text
            .spans
            .iter()
            .map(|span| (&text.text[span.range.clone()], span.style))
            .collect();
        let plain = Style::default();
        assert_eq!(
            styles,
            [
                (
                    "bold",
                    Style {
                        face: Face::Bold,
                        ..plain
                    }
                ),
                ("code", Style { code: true, ..plain }),
                (
                    "gone",
                    Style {
                        strike: true,
                        ..plain
                    }
                ),
                ("link", Style { link: true, ..plain }),
            ]
        );
        assert_eq!(text.links.len(), 1);
        assert_eq!(&text.text[text.links[0].range.clone()], "link");
        assert_eq!(text.links[0].url, "https://x.y");
    }

    #[test]
    fn an_item_of_only_bold_text_keeps_it_over_its_nested_list() {
        let blocks = parse("- **Exception.** text\n- **Docs.**\n  - deep one\n  - deep two\n");
        let [Block::List(items)] = &blocks[..] else {
            panic!("one list expected, got {blocks:#?}");
        };
        let [Block::Paragraph(text), Block::List(deep)] = &items[1].blocks[..] else {
            panic!("a text and a list expected, got {:#?}", items[1].blocks);
        };
        assert_eq!(text.text, "Docs.");
        assert_eq!(deep.len(), 2);
    }

    #[test]
    fn a_line_break_in_a_paragraph_is_a_space_or_stays() {
        let text = "one\ntwo\n\nthree";
        let [Block::Paragraph(joined), Block::Paragraph(_)] = &parse(text)[..] else {
            panic!("2 paragraphs expected");
        };
        assert_eq!(joined.text, "one two");
        let [Block::Paragraph(kept), Block::Paragraph(_)] = &parse_lines(text, true)[..] else {
            panic!("2 paragraphs expected");
        };
        assert_eq!(kept.text, "one\ntwo");
    }

    #[test]
    fn the_blocks_of_a_reply_come_out() {
        let text = "# Title\n\ntext\n\n- one\n- two\n  - deep\n\n1. first\n2. second\n\n- [ ] open\n- [x] done\n\n> quoted\n\n```rust\nfn main() {}\n```\n\n| A | B |\n|---|---|\n| 1 | 2 |\n\n---\n";
        let blocks = parse(text);
        let [
            Block::Heading {
                level: 1,
                text: title,
            },
            Block::Paragraph(_),
            Block::List(bullets),
            Block::List(numbers),
            Block::List(tasks),
            Block::Quote(quote),
            Block::Code { language, text: code },
            Block::Table { head, rows },
            Block::Rule,
        ] = &blocks[..]
        else {
            panic!("other blocks than expected: {blocks:#?}");
        };
        assert_eq!(title.text, "Title");
        assert_eq!(bullets.len(), 2);
        assert_eq!(bullets[0].marker, Marker::Bullet);
        // The nested list sits inside its item, after the text of it.
        assert!(
            matches!(&bullets[1].blocks[..], [Block::Paragraph(_), Block::List(deep)] if deep.len() == 1)
        );
        assert_eq!(numbers[1].marker, Marker::Number(2));
        assert_eq!(tasks[0].marker, Marker::Task(false));
        assert_eq!(tasks[1].marker, Marker::Task(true));
        assert!(matches!(&quote[..], [Block::Paragraph(text)] if text.text == "quoted"));
        assert_eq!(language, "rust");
        assert_eq!(code, "fn main() {}");
        assert_eq!(head.len(), 2);
        assert_eq!(rows[0][1].text, "2");
    }
}
