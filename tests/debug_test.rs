use dots_terminal_filter::command_parser::{CommandParser, CommandElement};

fn main() {
    let parser = CommandParser::new(true);
    let result = parser.parse("echo $(date)").unwrap();
    
    println!("Elements count: {}", result.elements.len());
    for (i, element) in result.elements.iter().enumerate() {
        println!("Element {}: {:?}", i, element);
    }
}
