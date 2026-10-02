use prettytable::{format, Cell, Row, Table};

type Extractor<T> = Box<dyn Fn(&T) -> String>;

pub struct TableMaker<T> {
    titles: Vec<String>,
    extractors: Vec<Extractor<T>>,
}

impl<T> TableMaker<T> {
    pub fn new() -> Self {
        TableMaker {
            titles: vec![],
            extractors: vec![],
        }
    }

    pub fn titles(&mut self, titles: Vec<String>) {
        self.titles = titles;
    }

    pub fn static_titles(&mut self, titles: Vec<&'static str>) {
        self.titles = titles.into_iter().map(|s| s.to_string()).collect();
    }

    pub fn with(&mut self, extractor: Extractor<T>) -> &mut Self {
        self.extractors.push(extractor);
        self
    }

    pub fn into_table(self, rows: &[T]) -> Table {
        let mut table = Table::new();

        let format = format::FormatBuilder::new().padding(0, 6).build();
        table.set_format(format);

        let TableMaker { titles, extractors } = self;

        let titles = titles
            .into_iter()
            .map(|t| Cell::new(&t).style_spec("b"))
            .collect();
        table.set_titles(Row::new(titles));

        rows.iter()
            .map(|t| {
                extractors
                    .iter()
                    .map(|e| e(t))
                    .map(|s| Cell::new(&s))
                    .collect()
            })
            .map(Row::new)
            .for_each(|row| {
                table.add_row(row);
            });

        table
    }
}
