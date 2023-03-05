# Finch Browser
## Usage Instructions

### Setup
1. Clone the repository to your local machine.
2. Run `cargo build` to build the Finch Browser project.

### Running Finch Browser
1. To run Finch Browser, use the `cargo run` command in your terminal.
2. By default, the browser will load the `test.html` and `test.css` files from the `examples` directory. To load your own files, use the `--html` and `--css` flags followed by the file paths:

        cargo run -- --html /path/to/your/file.html --css /path/to/your/file.css
        
4. To save the rendered page as a PNG image, the output file will be named `output.png` by default. To change the output filename, use the `-o` option followed by your desired filename: 

        cargo run -- -o output_file_name.png
        
4. To save the rendered page as a PDF file instead of a PNG, add the `--format pdf` flag to the command: 
    
        cargo run -- --format pdf


By default, the output file will be named `output.pdf`. To change the filename, use the `-o` option followed by your desired filename.
