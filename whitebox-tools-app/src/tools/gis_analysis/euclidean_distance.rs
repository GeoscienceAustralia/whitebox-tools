/*
This tool is part of the WhiteboxTools geospatial analysis library.
Authors: Dr. John Lindsay
Created: 22/06/2017
Last Modified: 05/12/2019
License: MIT
*/

use whitebox_raster::*;
use whitebox_common::structures::Array2D;
use crate::tools::*;
use std::env;
use std::f64;
use std::io::{Error, ErrorKind};
use std::path;

/// This tool will estimate the Euclidean distance (i.e. straight-line distance) between each
/// grid cell and the nearest 'target cell' in the input image. Target cells are all non-zero,
/// non-NoData grid cells. Distance in the output image is measured in the same units as the
/// horizontal units of the input image.
///
/// # Algorithm Description
/// The algorithm is based on the highly efficient distance transform of Shih and Wu (2003).
/// It makes four passes of the image; the first pass initializes the output image; the second
/// and third passes calculate the minimum squared Euclidean distance by examining the 3 x 3
/// neighbourhood surrounding each cell; the last pass takes the square root of cell values,
/// transforming them into true Euclidean distances, and deals with NoData values that may be
/// present. All NoData value grid cells in the input image will contain NoData values in the
/// output image. As such, NoData is not a suitable background value for non-target cells.
/// Background areas should be designated with zero values.
///
/// # Reference
/// Shih FY and Wu Y-T (2004), Fast Euclidean distance transformation in two scans using a 3 x 3
/// neighborhood, *Computer Vision and Image Understanding*, 93: 195-205.
///
/// # See Also
/// `EuclideanAllocation`, `CostDistance`
pub struct EuclideanDistance {
    name: String,
    description: String,
    toolbox: String,
    parameters: Vec<ToolParameter>,
    example_usage: String,
}

impl EuclideanDistance {
    pub fn new() -> EuclideanDistance {
        // public constructor
        let name = "EuclideanDistance".to_string();
        let toolbox = "GIS Analysis/Distance Tools".to_string();
        let description =
            "Calculates the Shih and Wu (2004) Euclidean distance transform.".to_string();

        let mut parameters = vec![];
        parameters.push(ToolParameter {
            name: "Input File".to_owned(),
            flags: vec!["-i".to_owned(), "--input".to_owned()],
            description: "Input raster file.".to_owned(),
            parameter_type: ParameterType::ExistingFile(ParameterFileType::Raster),
            default_value: None,
            optional: false,
        });

        parameters.push(ToolParameter {
            name: "Output File".to_owned(),
            flags: vec!["-o".to_owned(), "--output".to_owned()],
            description: "Output raster file.".to_owned(),
            parameter_type: ParameterType::NewFile(ParameterFileType::Raster),
            default_value: None,
            optional: false,
        });

        let sep: String = path::MAIN_SEPARATOR.to_string();
        let e = format!("{}", env::current_exe().unwrap().display());
        let mut parent = env::current_exe().unwrap();
        parent.pop();
        let p = format!("{}", parent.display());
        let mut short_exe = e
            .replace(&p, "")
            .replace(".exe", "")
            .replace(".", "")
            .replace(&sep, "");
        if e.contains(".exe") {
            short_exe += ".exe";
        }
        let usage = format!(
            ">>.*{} -r={} -v --wd=\"*path*to*data*\" -i=DEM.tif -o=output.tif",
            short_exe, name
        )
        .replace("*", &sep);

        EuclideanDistance {
            name: name,
            description: description,
            toolbox: toolbox,
            parameters: parameters,
            example_usage: usage,
        }
    }
}

impl WhiteboxTool for EuclideanDistance {
    fn get_source_file(&self) -> String {
        String::from(file!())
    }

    fn get_tool_name(&self) -> String {
        self.name.clone()
    }

    fn get_tool_description(&self) -> String {
        self.description.clone()
    }

    fn get_tool_parameters(&self) -> String {
        match serde_json::to_string(&self.parameters) {
            Ok(json_str) => return format!("{{\"parameters\":{}}}", json_str),
            Err(err) => return format!("{:?}", err),
        }
    }

    fn get_example_usage(&self) -> String {
        self.example_usage.clone()
    }

    fn get_toolbox(&self) -> String {
        self.toolbox.clone()
    }

    fn run<'a>(
        &self,
        args: Vec<String>,
        working_directory: &'a str,
        verbose: bool,
    ) -> Result<(), Error> {
        let mut input_file = String::new();
        let mut output_file = String::new();

        if args.len() == 0 {
            return Err(Error::new(
                ErrorKind::InvalidInput,
                "Tool run with no parameters.",
            ));
        }
        for i in 0..args.len() {
            let mut arg = args[i].replace("\"", "");
            arg = arg.replace("\'", "");
            let cmd = arg.split("="); // in case an equals sign was used
            let vec = cmd.collect::<Vec<&str>>();
            let mut keyval = false;
            if vec.len() > 1 {
                keyval = true;
            }
            let flag_val = vec[0].to_lowercase().replace("--", "-");
            if flag_val == "-i" || flag_val == "-input" {
                input_file = if keyval {
                    vec[1].to_string()
                } else {
                    args[i + 1].to_string()
                };
            } else if flag_val == "-o" || flag_val == "-output" {
                output_file = if keyval {
                    vec[1].to_string()
                } else {
                    args[i + 1].to_string()
                };
            }
        }

        if verbose {
            let tool_name = self.get_tool_name();
            let welcome_len = format!("* Welcome to {} *", tool_name).len().max(28); 
            // 28 = length of the 'Powered by' by statement.
            println!("{}", "*".repeat(welcome_len));
            println!("* Welcome to {} {}*", tool_name, " ".repeat(welcome_len - 15 - tool_name.len()));
            println!("* Powered by WhiteboxTools {}*", " ".repeat(welcome_len - 28));
            println!("* www.whiteboxgeo.com {}*", " ".repeat(welcome_len - 23));
            println!("{}", "*".repeat(welcome_len));
        }

        let sep: String = path::MAIN_SEPARATOR.to_string();

        let mut progress: usize;
        let mut old_progress: usize = 1;

        if !input_file.contains(&sep) && !input_file.contains("/") {
            input_file = format!("{}{}", working_directory, input_file);
        }
        if !output_file.contains(&sep) && !output_file.contains("/") {
            output_file = format!("{}{}", working_directory, output_file);
        }

        if verbose {
            println!("Reading data...")
        };

        let input = Raster::new(&input_file, "r")?;

        let nodata = input.configs.nodata;
        let rows = input.configs.rows as isize;
        let columns = input.configs.columns as isize;

        let start = Instant::now();

        let mut rx: Array2D<f64> = Array2D::new(rows, columns, 0f64, nodata)?;
        let mut ry: Array2D<f64> = Array2D::new(rows, columns, 0f64, nodata)?;

        let mut output = Raster::initialize_using_file(&output_file, &input);
        output.configs.data_type = DataType::F32;

        let mut h: f64;
        let mut which_cell: usize;
        let inf_val = f64::INFINITY;
        let dx = [-1, -1, 0, 1, 1, 1, 0, -1];
        let dy = [0, -1, -1, -1, 0, 1, 1, 1];
        let gx = [1.0, 1.0, 0.0, 1.0, 1.0, 1.0, 0.0, 1.0];
        let gy = [0.0, 1.0, 1.0, 1.0, 0.0, 1.0, 1.0, 1.0];
        let (mut x, mut y): (isize, isize);
        let (mut z, mut z2, mut z_min): (f64, f64, f64);

        for row in 0..rows {
            for col in 0..columns {
                if input.get_value(row, col) != 0.0 {
                    output.set_value(row, col, 0.0);
                } else {
                    output.set_value(row, col, inf_val);
                }
            }
            if verbose {
                progress = (100.0_f64 * row as f64 / (rows - 1) as f64) as usize;
                if progress != old_progress {
                    println!("Initializing Rasters: {}%", progress);
                    old_progress = progress;
                }
            }
        }

        for row in 0..rows {
            for col in 0..columns {
                z = output.get_value(row, col);
                if z != 0.0 {
                    z_min = inf_val;
                    which_cell = 0;
                    for i in 0..4 {
                        x = col + dx[i];
                        y = row + dy[i];
                        z2 = output.get_value(y, x);
                        if z2 != nodata {
                            h = match i {
                                0 => 2.0 * rx.get_value(y, x) + 1.0,
                                1 => 2.0 * (rx.get_value(y, x) + ry.get_value(y, x) + 1.0),
                                2 => 2.0 * ry.get_value(y, x) + 1.0,
                                _ => 2.0 * (rx.get_value(y, x) + ry.get_value(y, x) + 1.0), // 3
                            };
                            z2 += h;
                            if z2 < z_min {
                                z_min = z2;
                                which_cell = i;
                            }
                        }
                    }
                    if z_min < z {
                        output.set_value(row, col, z_min);
                        x = col + dx[which_cell];
                        y = row + dy[which_cell];
                        rx.set_value(row, col, rx.get_value(y, x) + gx[which_cell]);
                        ry.set_value(row, col, ry.get_value(y, x) + gy[which_cell]);
                    }
                }
            }
            if verbose {
                progress = (100.0_f64 * row as f64 / (rows - 1) as f64) as usize;
                if progress != old_progress {
                    println!("Progress (1 of 3): {}%", progress);
                    old_progress = progress;
                }
            }
        }

        for row in (0..rows).rev() {
            for col in (0..columns).rev() {
                z = output.get_value(row, col);
                if z != 0.0 {
                    z_min = inf_val;
                    which_cell = 0;
                    for i in 4..8 {
                        x = col + dx[i];
                        y = row + dy[i];
                        z2 = output.get_value(y, x);
                        if z2 != nodata {
                            h = match i {
                                5 => 2.0 * (rx.get_value(y, x) + ry.get_value(y, x) + 1.0),
                                4 => 2.0 * rx.get_value(y, x) + 1.0,
                                6 => 2.0 * ry.get_value(y, x) + 1.0,
                                _ => 2.0 * (rx.get_value(y, x) + ry.get_value(y, x) + 1.0), // 7
                            };
                            z2 += h;
                            if z2 < z_min {
                                z_min = z2;
                                which_cell = i;
                            }
                        }
                    }
                    if z_min < z {
                        output[(row, col)] = z_min;
                        x = col + dx[which_cell];
                        y = row + dy[which_cell];
                        rx.set_value(row, col, rx.get_value(y, x) + gx[which_cell]);
                        ry.set_value(row, col, ry.get_value(y, x) + gy[which_cell]);
                    }
                }
            }
            if verbose {
                progress = (100.0_f64 * (rows - row) as f64 / (rows - 1) as f64) as usize;
                if progress != old_progress {
                    println!("Progress (2 of 3): {}%", progress);
                    old_progress = progress;
                }
            }
        }

        let cell_size = (input.configs.resolution_x + input.configs.resolution_y) / 2.0;
        for row in 0..rows {
            for col in 0..columns {
                if input.get_value(row, col) != nodata {
                    output.set_value(row, col, output.get_value(row, col).sqrt() * cell_size);
                } else {
                    output.set_value(row, col, nodata);
                }
            }
            if verbose {
                progress = (100.0_f64 * row as f64 / (rows - 1) as f64) as usize;
                if progress != old_progress {
                    println!("Progress (3 of 3): {}%", progress);
                    old_progress = progress;
                }
            }
        }

        let elapsed_time = get_formatted_elapsed_time(start);
        output.configs.palette = "spectrum.plt".to_string();
        output.add_metadata_entry(format!(
            "Created by whitebox_tools\' {} tool",
            self.get_tool_name()
        ));
        output.add_metadata_entry(format!("Input file: {}", input_file));
        output.add_metadata_entry(format!("Elapsed Time (excluding I/O): {}", elapsed_time));

        if verbose {
            println!("Saving data...")
        };
        let _ = match output.write() {
            Ok(_) => {
                if verbose {
                    println!("Output file written")
                }
            }
            Err(e) => return Err(e),
        };

        if verbose {
            println!(
                "{}",
                &format!("Elapsed Time (excluding I/O): {}", elapsed_time)
            );
        }

        Ok(())
    }
}
