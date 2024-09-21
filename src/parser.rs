use markdown::{Constructs, ParseOptions};

pub fn get_parser_options() -> ParseOptions {
    ParseOptions {
        constructs: Constructs {
            autolink: false,
            code_indented: false,
            gfm_footnote_definition: true,
            gfm_label_start_footnote: true,
            gfm_table: true,
            html_flow: false,
            html_text: false,
            mdx_esm: true,
            mdx_expression_flow: true,
            mdx_expression_text: true,
            mdx_jsx_flow: true,
            mdx_jsx_text: true,
            ..Default::default()
        },
        ..Default::default()
    }
}
