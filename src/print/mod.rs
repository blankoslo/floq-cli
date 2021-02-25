use prettytable::{Cell, Row, Table, format};

pub struct TableMaker<'t, T> {
    titles: Vec<&'static str>,
    extractors: Vec<fn(&'t T) -> String>,
}

impl<'t, T>  TableMaker<'t, T> {

    pub fn new(titles: Vec<&'static str>,) -> Self {
        TableMaker { titles, extractors: vec![] }
    }

    pub fn with(&mut self, extractor: fn(&'t T) -> String) -> &mut Self {
        self.extractors.push(extractor);
        self
    }

    pub fn into_table(self, rows: &'t [T]) -> Table {
        let mut table = Table::new();

        let format = format::FormatBuilder::new()
            .padding(0, 6)
            .build();
        table.set_format(format);

        let TableMaker{titles, extractors} = self;

        let titles = titles.into_iter()
            .map(|t| Cell::new(t).style_spec("b"))
            .collect();
        table.set_titles(Row::new(titles));

        rows.iter()
            .map(|t| {
                extractors.iter()
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