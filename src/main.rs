mod classification;
mod dependency_types;
mod matrix_generation;
mod parser;

use classification::{classify_matrix, Classification as MatrixClassification, InputMatrix, CalculatedPercentages};
use matrix_generation::generate_dependency_matrix;
use parser::parse_into_traces;

use std::rc::Rc;
use wasm_bindgen::prelude::*;
use wasm_bindgen_futures::spawn_local;
use web_sys::{File, HtmlInputElement, MouseEvent, ProgressEvent, FileReader, Event, InputEvent};
use yew::prelude::*;
use clap::Parser;

#[derive(Parser, Debug)]
#[clap(author, version, about, long_about = None)]
struct Args {
    #[clap(short, long, value_parser)]
    file_path: Option<String>,

    #[clap(long)]
    print_ratios: bool,

    #[clap(long, default_value_t = 1.0)]
    temporal_threshold: f64,

    #[clap(long, default_value_t = 1.0)]
    existential_threshold: f64,
}

#[derive(Debug, thiserror::Error, Clone, PartialEq)]
enum AppError {
    #[error("File reading error: {0}")]
    FileReadError(String),
    #[error("XES parsing error: {0}")]
    XesParseError(String),
    #[error("Classification error: {0}")]
    ClassificationError(String),
}

enum AppMessage {
    FileSelected(Option<File>),
    FileLoaded(Result<String, String>),
    ExistentialThresholdChanged(String),
    TemporalThresholdChanged(String),
    ProcessLog,
    SetClassificationResult(Result<MatrixClassification, AppError>),
}

#[derive(Clone, PartialEq)]
struct AppState {
    file_name: Option<String>,
    file_content: Option<String>,
    existential_threshold_str: String, // Store as String
    temporal_threshold_str: String,    // Store as String
    classification_result: Option<Result<MatrixClassification, AppError>>,
    is_processing: bool,
}

impl Default for AppState {
    fn default() -> Self {
        Self {
            file_name: None,
            file_content: None,
            existential_threshold_str: "1.0".to_string(), // Default to "1.0" string
            temporal_threshold_str: "1.0".to_string(),    // Default to "1.0" string
            classification_result: None,
            is_processing: false,
        }
    }
}

fn parse_threshold_str(s: &str) -> Option<f64> {
    s.parse::<f64>().ok().filter(|&val| (0.0..=1.0).contains(&val))
}

#[function_component(App)]
fn app() -> Html {
    let app_state_handle: UseStateHandle<AppState> = use_state(AppState::default);

    let dispatch = {
        let app_state_handle = app_state_handle.clone();
        Rc::new(move |msg: AppMessage| {
            let mut new_state = (*app_state_handle).clone();
            match msg {
                AppMessage::FileSelected(file_opt) => {
                    if let Some(file) = file_opt {
                        new_state.file_name = Some(file.name());
                        new_state.file_content = None;
                        new_state.classification_result = None;
                    } else {
                        new_state.file_name = None;
                        new_state.file_content = None;
                        new_state.classification_result = None;
                    }
                }
                AppMessage::FileLoaded(result) => {
                    match result {
                        Ok(content) => new_state.file_content = Some(content),
                        Err(err_msg) => {
                            new_state.file_content = None;
                            new_state.classification_result = Some(Err(AppError::FileReadError(err_msg)));
                        }
                    }
                }
                AppMessage::ExistentialThresholdChanged(val_str) => {
                    new_state.existential_threshold_str = val_str; // Just update string
                }
                AppMessage::TemporalThresholdChanged(val_str) => {
                    new_state.temporal_threshold_str = val_str; // Just update string
                }
                AppMessage::ProcessLog => {
                    new_state.is_processing = true;
                    new_state.classification_result = None;
                }
                AppMessage::SetClassificationResult(result) => {
                    new_state.classification_result = Some(result);
                    new_state.is_processing = false;
                }
            }
            app_state_handle.set(new_state);
        })
    };

    let on_file_change = {
        let dispatch = dispatch.clone();
        Callback::from(move |e: Event| {
            let input: HtmlInputElement = e.target_unchecked_into();
            if let Some(files) = input.files() {
                if let Some(file) = files.get(0) {
                    dispatch(AppMessage::FileSelected(Some(file.clone())));
                    
                    let reader = FileReader::new().expect("Failed to create FileReader");
                    let dispatch_clone_for_load = dispatch.clone();
                    let onload = Closure::wrap(Box::new(move |event: ProgressEvent| {
                        let reader_res: FileReader = event.target_unchecked_into();
                        match reader_res.result() {
                            Ok(content_val) => {
                                if let Some(content_str) = content_val.as_string() {
                                    dispatch_clone_for_load(AppMessage::FileLoaded(Ok(content_str)));
                                } else {
                                    dispatch_clone_for_load(AppMessage::FileLoaded(Err("Content is not a string".to_string())));
                                }
                            }
                            Err(err) => {
                                dispatch_clone_for_load(AppMessage::FileLoaded(Err(format!("{:?}", err))));
                            }
                        }
                    }) as Box<dyn FnMut(ProgressEvent)>);
                    reader.set_onload(Some(onload.as_ref().unchecked_ref()));
                    if reader.read_as_text(&file).is_err() {
                         dispatch(AppMessage::FileLoaded(Err("Failed to initiate file read".to_string())));
                    }
                    onload.forget();
                } else {
                     dispatch(AppMessage::FileSelected(None));
                }
            }
        })
    };
    
    let on_existential_threshold_change = {
        let dispatch = dispatch.clone();
        Callback::from(move |e: InputEvent| {
            let input: HtmlInputElement = e.target_unchecked_into();
            dispatch(AppMessage::ExistentialThresholdChanged(input.value()));
        })
    };

    let on_temporal_threshold_change = {
        let dispatch = dispatch.clone();
        Callback::from(move |e: InputEvent| {
            let input: HtmlInputElement = e.target_unchecked_into();
            dispatch(AppMessage::TemporalThresholdChanged(input.value()));
        })
    };

    let on_process_log = {
        let app_state_snapshot = (*app_state_handle).clone();
        let dispatch = dispatch.clone();
        Callback::from(move |_mouse_event: MouseEvent| {
            // Parse and validate thresholds at the point of processing
            let temp_thresh_opt = parse_threshold_str(&app_state_snapshot.temporal_threshold_str);
            let ex_thresh_opt = parse_threshold_str(&app_state_snapshot.existential_threshold_str);

            if app_state_snapshot.file_content.is_some() && 
               !app_state_snapshot.is_processing &&
               temp_thresh_opt.is_some() && // Crucial check here
               ex_thresh_opt.is_some()     // Crucial check here
            {
                dispatch(AppMessage::ProcessLog);
                
                let content_clone = app_state_snapshot.file_content.clone().unwrap();
                let temp_thresh_val = temp_thresh_opt.unwrap(); // Safe due to check above
                let ex_thresh_val = ex_thresh_opt.unwrap();   // Safe due to check above
                
                let dispatch_for_result = dispatch.clone();

                spawn_local(async move {
                    let classification_task_result = async move {
                        let traces = parse_into_traces(None, Some(&content_clone))
                            .map_err(|e| AppError::XesParseError(e.to_string()))?;
                        
                        if traces.is_empty() {
                            return Err(AppError::XesParseError("No traces found in log.".to_string()));
                        }

                        let matrix: InputMatrix = generate_dependency_matrix(
                            &traces,
                            ex_thresh_val,
                            temp_thresh_val,
                        );
                        
                        match classify_matrix(&matrix) {
                            MatrixClassification::Error(e_str) => Err(AppError::ClassificationError(e_str)),
                            other_classification => Ok(other_classification),
                        }
                    }.await;
                    dispatch_for_result(AppMessage::SetClassificationResult(classification_task_result));
                });
            }
        })
    };

    let current_app_state_for_view = (*app_state_handle).clone();

    // Determine button disabled state for the view
    let is_temporal_thresh_valid = parse_threshold_str(&current_app_state_for_view.temporal_threshold_str).is_some();
    let is_existential_thresh_valid = parse_threshold_str(&current_app_state_for_view.existential_threshold_str).is_some();
    let is_process_button_disabled = current_app_state_for_view.file_content.is_none() || 
                                     current_app_state_for_view.is_processing ||
                                     !is_temporal_thresh_valid ||
                                     !is_existential_thresh_valid;

    html! {
        <div class="container" style="padding: 20px; font-family: sans-serif;">
            <h1>{ "Event Log Classifier" }</h1>

            <div class="controls" style="margin-bottom: 20px; display: flex; gap: 20px; align-items: center;">
                <div>
                    <label for="xes-file" style="margin-right: 5px;">{ "Upload XES File:" }</label>
                    <input type="file" id="xes-file" accept=".xes" onchange={on_file_change} />
                    if let Some(name) = current_app_state_for_view.file_name {
                        <p style="font-size: 0.9em; margin-top: 5px;">{ format!("Selected: {}", name) }</p>
                    }
                </div>
            </div>

            <div class="thresholds" style="margin-bottom: 20px; display: flex; gap: 30px;">
                <div>
                    <label for="temporal-threshold" style="margin-right: 5px;">{ "Temporal Threshold (0.0-1.0):" }</label>
                    <input
                        id="temporal-threshold"
                        type="number" 
                        min="0.0" max="1.0" step="0.05"
                        value={current_app_state_for_view.temporal_threshold_str.clone()} // Bind to string state
                        oninput={on_temporal_threshold_change}
                        style={if !is_temporal_thresh_valid && !current_app_state_for_view.temporal_threshold_str.is_empty() {"width: 70px; border-color: red;"} else {"width: 70px;"} }
                    />
                </div>
                <div>
                    <label for="existential-threshold" style="margin-right: 5px;">{ "Existential Threshold (0.0-1.0):" }</label>
                    <input
                        id="existential-threshold"
                        type="number"
                        min="0.0" max="1.0" step="0.05"
                        value={current_app_state_for_view.existential_threshold_str.clone()} // Bind to string state
                        oninput={on_existential_threshold_change}
                        style={if !is_existential_thresh_valid && !current_app_state_for_view.existential_threshold_str.is_empty() {"width: 70px; border-color: red;"} else {"width: 70px;"} }
                    />
                </div>
            </div>

            <button
                onclick={on_process_log}
                disabled={is_process_button_disabled} // Use the new combined condition
                style="padding: 10px 15px; font-size: 1em; cursor: pointer;"
            >
                { if current_app_state_for_view.is_processing { "Processing..." } else { "Process Log" } }
            </button>

             <div class="results" style="margin-top: 30px;">
                <h2>{ "Classification Result" }</h2>
                {
                    if current_app_state_for_view.is_processing {
                        html! { <p>{ "Processing log, please wait..." }</p> }
                    } else {
                        match current_app_state_for_view.classification_result {
                            Some(Ok(classification)) => html! {
                                <p style="font-size: 1.2em; color: green;">{ format!("{:?}", classification) }</p>
                            },
                            Some(Err(err)) => html! {
                                <p style="font-size: 1.2em; color: red;">{ format!("Error: {}", err) }</p>
                            },
                            None => html! {
                                <p>{ "Upload and process a log to see results." }</p>
                            }
                        }
                    }
                }
            </div>

        </div>
    }
}

fn main() {
    let args = Args::parse();

    if args.file_path.is_some() {
        let file_path = args.file_path.unwrap();
        let temporal_threshold = args.temporal_threshold;
        let existential_threshold = args.existential_threshold;

        if !(0.0..=1.0).contains(&temporal_threshold) {
            eprintln!("Error: Temporal threshold must be between 0.0 and 1.0");
            std::process::exit(1);
        }

        if !(0.0..=1.0).contains(&existential_threshold) {
            eprintln!("Error: Existential threshold must be between 0.0 and 1.0");
            std::process::exit(1);
        }

        match parse_into_traces(Some(&file_path), None) {
            Ok(traces) => {
                let matrix = generate_dependency_matrix(&traces, existential_threshold, temporal_threshold);
                let classification_result = classify_matrix(&matrix);
                println!("Classification Result: {}", classification_result);

                if args.print_ratios {
                    match CalculatedPercentages::new(&matrix) {
                        Ok(percentages) => {
                            // TODO: implement Display trait for CalculatedPercentages
                            println!("
Calculated Percentages:");
                            println!("{:?}", percentages);
                        }
                        Err(e) => {
                            eprintln!("Error calculating percentages: {}", e);
                        }
                    }
                }
            }
            Err(e) => {
                eprintln!("Error parsing XES file: {}", e);
                std::process::exit(1);
            }
        }
    } else if args.print_ratios {
        eprintln!("Error: --file-path is required when using --print-ratios in CLI mode.");
        std::process::exit(1);
    } else {
        yew::Renderer::<App>::new().render();
    }
}