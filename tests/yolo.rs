#[cfg(feature = "inspection")]
pub mod test {
    use std::path::Path;
    use std::sync::Once;

    use lean_link::service::inspection::detector::Detector;
    use lean_link::service::inspection::image::InferenceImage;
    use lean_link::service::inspection::yolo::OnnxInference;

    const MODEL11_PATH: &str = r"C:\Users\fz_ka\Development\yolo\line1-11.onnx";
    const MODEL26_PATH: &str = r"C:\Users\fz_ka\Development\yolo\line1-26.onnx";
    const TEST_IMAGES_DIR: &str = r"C:\Users\fz_ka\Development\yolo\line1\val\images";

    static TRACING_INIT: Once = Once::new();

    fn init_tracing() {
        TRACING_INIT.call_once(|| {
            let _ = tracing_subscriber::fmt()
                .with_max_level(tracing::Level::INFO)
                .try_init();
        });
    }

    fn get_test_images() -> Vec<std::path::PathBuf> {
        let dir = Path::new(TEST_IMAGES_DIR);
        if !dir.exists() {
            panic!("Test images directory not found: {}", TEST_IMAGES_DIR);
        }

        let mut images: Vec<std::path::PathBuf> = std::fs::read_dir(dir)
            .expect("Failed to read images directory")
            .filter_map(|entry| {
                let entry = entry.ok()?;
                let path = entry.path();
                let ext = path.extension()?.to_str()?.to_lowercase();
                if ["jpg", "jpeg", "png", "bmp"].contains(&ext.as_str()) {
                    Some(path)
                } else {
                    None
                }
            })
            .collect();

        images.sort();
        images
    }

    /// Compare output format between YOLO11 and YOLO26
    fn diagnose_model(model_path: &str, model_name: &str) {
        println!("\n============================================");
        println!("=== Model: {} ===", model_name);
        println!("Path: {}", model_path);

        if !Path::new(model_path).exists() {
            println!("Model not found, skipping");
            return;
        }

        let mut inference = OnnxInference::new(model_path, model_name);
        if let Err(e) = inference.initialize() {
            println!("Failed to initialize: {:?}", e);
            return;
        }

        // Output info
        let output_infos = inference.get_output_infos();
        println!("\n--- Output Info ---");
        println!("Number of outputs: {}", output_infos.len());
        for (idx, info) in output_infos.iter().enumerate() {
            println!(
                "Output {}: name={}, dimensions={:?}",
                idx, info.name, info.dimensions
            );
        }

        // Analyze output dimensions
        if !output_infos.is_empty() {
            let dims = &output_infos[0].dimensions;
            println!("\n--- Dimension Analysis ---");
            if dims.len() == 3 {
                let batch = dims[0];
                let dim1 = dims[1];
                let dim2 = dims[2];
                println!("Shape: [{}, {}, {}]", batch, dim1, dim2);
                
                // Standard YOLO format: [1, 4+num_classes, num_detections]
                // dim1 should be small (like 8 for 4 classes), dim2 should be large (like 8400)
                if dim1 < dim2 {
                    println!("Format: STANDARD [1, 4+classes, detections]");
                    println!("  - Num classes: {}", dim1 - 4);
                    println!("  - Num detections: {}", dim2);
                } else if dim2 == 6 {
                    // One-to-One format (YOLOv10/YOLO11 one2one): [1, num_detections, 6]
                    // Each detection: [x1, y1, x2, y2, confidence, class_id]
                    println!("Format: ONE-TO-ONE [1, detections, 6]");
                    println!("  - Max detections: {}", dim1);
                    println!("  - No NMS required, results pre-sorted by confidence");
                } else {
                    println!("Format: TRANSPOSED [1, detections, 4+classes]");
                    println!("  - Num detections: {}", dim1);
                    println!("  - Num classes: {}", dim2 - 4);
                }
            }
        }

        // Class names
        let class_names = inference.get_class_names();
        println!("\n--- Class Names ---");
        println!("Count: {}", class_names.len());
        println!("Names: {:?}", class_names);

        // Task and imgsz
        if let Some(task) = inference.get_task() {
            println!("Task: {}", task);
        }
        if let Some((h, w)) = inference.get_imgsz() {
            println!("Image size: {}x{}", h, w);
        }

        // Test detection on first image
        let images = get_test_images();
        if !images.is_empty() {
            let image = InferenceImage::from_file(&images[0]).expect("Failed to load image");
            println!("\n--- Detection Test ---");
            println!("Test image: {}x{}", image.width, image.height);

            let result = inference.detect(&image).expect("Detection failed");
            println!("Detections: {}", result.detections.len());
            println!("Is OK: {}", result.is_ok);
            println!("Processing time: {}ms", result.processing_time_ms);

            for (i, det) in result.detections.iter().enumerate() {
                println!(
                    "  [{}] {} (class {}): conf={:.3}, bbox={:?}",
                    i, det.class_name, det.class_id, det.confidence, det.bbox
                );
            }

            if result.detections.is_empty() {
                println!("*** WARNING: No detections found! ***");
            }
        }
    }

    #[test]
    fn test_compare_yolo11_vs_yolo26() {
        init_tracing();

        println!("\n");
        println!("############################################");
        println!("# Comparing YOLO11 vs YOLO26 Output Format #");
        println!("############################################");

        diagnose_model(MODEL11_PATH, "YOLO11");
        diagnose_model(MODEL26_PATH, "YOLO26");

        println!("\n============================================");
        println!("=== Analysis Summary ===");

        // Load both models and compare dimensions
        if Path::new(MODEL11_PATH).exists() && Path::new(MODEL26_PATH).exists() {
            let mut inf11 = OnnxInference::new(MODEL11_PATH, "yolo11");
            let mut inf26 = OnnxInference::new(MODEL26_PATH, "yolo26");

            if inf11.initialize().is_ok() && inf26.initialize().is_ok() {
                let out11 = inf11.get_output_infos();
                let out26 = inf26.get_output_infos();

                println!("\n--- Dimension Comparison ---");
                if !out11.is_empty() && !out26.is_empty() {
                    let d11 = &out11[0].dimensions;
                    let d26 = &out26[0].dimensions;
                    println!("YOLO11 dimensions: {:?}", d11);
                    println!("YOLO26 dimensions: {:?}", d26);

                    if d11 != d26 {
                        println!("\n--- Format Difference ---");
                        println!("YOLO11: Standard format [1, 4+classes, detections]");
                        println!("YOLO26: One-to-One format [1, max_detections, 6]");
                        println!("\nBoth formats are now supported!");
                    } else {
                        println!("Dimensions are identical.");
                    }
                }
            }
        }
    }

    #[test]
    fn test_onnx_model_initialization() {
        if !Path::new(MODEL11_PATH).exists() {
            eprintln!("Skipping test: Model not found at {}", MODEL11_PATH);
            return;
        }

        let mut inference = OnnxInference::new(MODEL11_PATH, "test_model");
        let result = inference.initialize();
        assert!(
            result.is_ok(),
            "Failed to initialize model: {:?}",
            result.err()
        );

        // Check model info
        let input_info = inference.get_input_info().expect("Should have input info");
        println!(
            "Model input: {}x{}x{} (CxHxW)",
            input_info.channels, input_info.height, input_info.width
        );

        // Check class names loaded from metadata
        let class_names = inference.get_class_names();
        println!("Class names: {:?}", class_names);
        assert!(
            !class_names.is_empty(),
            "Class names should be loaded from model metadata"
        );

        // Check task from metadata
        if let Some(task) = inference.get_task() {
            println!("Task from metadata: {}", task);
        }

        // Check imgsz from metadata
        if let Some((h, w)) = inference.get_imgsz() {
            println!("Image size from metadata: [{}, {}]", h, w);
        }
    }

    #[test]
    fn test_onnx_detection_single_image() {
        if !Path::new(MODEL11_PATH).exists() {
            eprintln!("Skipping test: Model not found at {}", MODEL11_PATH);
            return;
        }

        let images = get_test_images();
        if images.is_empty() {
            eprintln!("Skipping test: No test images found in {}", TEST_IMAGES_DIR);
            return;
        }

        // Use first image for testing
        let test_image = &images[0];
        println!("Testing with image: {:?}", test_image);

        // Initialize model
        let mut inference = OnnxInference::new(MODEL11_PATH, "test_detection");
        inference.initialize().expect("Failed to initialize model");

        // Load image
        let image = InferenceImage::from_file(test_image).expect("Failed to load image");
        println!(
            "Image size: {}x{}, channels: {}",
            image.width, image.height, image.channels
        );

        // Run detection
        let result = inference.detect(&image).expect("Detection failed");

        println!("Detection result:");
        println!("  - Total detections: {}", result.detections.len());
        println!("  - Is OK: {}", result.is_ok);
        println!("  - Processing time: {}ms", result.processing_time_ms);

        for (i, det) in result.detections.iter().enumerate() {
            println!(
                "  [{}] {} (class {}): confidence {:.3}, bbox: {:?}",
                i, det.class_name, det.class_id, det.confidence, det.bbox
            );
        }
    }

    #[test]
    fn test_onnx_detection_all_images() {
        init_tracing();
        if !Path::new(MODEL11_PATH).exists() {
            eprintln!("Skipping test: Model not found at {}", MODEL11_PATH);
            return;
        }

        let images = get_test_images();
        if images.is_empty() {
            eprintln!("Skipping test: No test images found in {}", TEST_IMAGES_DIR);
            return;
        }

        // Initialize model once
        let mut inference = OnnxInference::new(MODEL11_PATH, "test_batch");
        inference.initialize().expect("Failed to initialize model");

        println!("\n=== Testing {} images ===\n", images.len());

        let mut total_time = 0u64;
        let mut total_detections = 0usize;

        for image_path in &images {
            let image_name = image_path.file_name().unwrap().to_str().unwrap();

            let image = match InferenceImage::from_file(image_path) {
                Ok(img) => img,
                Err(e) => {
                    println!("  [SKIP] {}: Failed to load: {}", image_name, e);
                    continue;
                }
            };

            let result = inference.detect(&image).expect("Detection failed");

            total_time += result.processing_time_ms;
            total_detections += result.detections.len();

            let status = if result.is_ok { "OK" } else { "NG" };
            println!(
                "  {} -> {} detections, {}ms [{}]",
                image_name,
                result.detections.len(),
                result.processing_time_ms,
                status
            );
        }

        println!("\n=== Summary ===");
        println!("Total images: {}", images.len());
        println!("Total detections: {}", total_detections);
        println!("Total time: {}ms", total_time);
        println!(
            "Average time: {:.2}ms",
            total_time as f64 / images.len() as f64
        );
    }

    /// Benchmark inference time comparison between YOLO11 and YOLO26
    fn benchmark_model(model_path: &str, model_name: &str, images: &[std::path::PathBuf]) -> (u64, usize) {
        if !Path::new(model_path).exists() {
            println!("{} model not found, skipping", model_name);
            return (0, 0);
        }

        let mut inference = OnnxInference::new(model_path, model_name);
        inference.initialize().expect("Failed to initialize model");

        println!("\n=== {} Benchmark ===", model_name);

        let mut total_time = 0u64;
        let mut total_detections = 0usize;

        // Warmup: first run to initialize cache
        if !images.is_empty() {
            let warmup_image = InferenceImage::from_file(&images[0]).expect("Failed to load warmup image");
            let _ = inference.detect(&warmup_image);
            println!("Warmup completed");
        }

        // Benchmark runs
        for image_path in images {
            let image_name = image_path.file_name().unwrap().to_str().unwrap();

            let image = match InferenceImage::from_file(image_path) {
                Ok(img) => img,
                Err(e) => {
                    println!("  [SKIP] {}: {}", image_name, e);
                    continue;
                }
            };

            let result = inference.detect(&image).expect("Detection failed");
            total_time += result.processing_time_ms;
            total_detections += result.detections.len();
        }

        let avg_time = if !images.is_empty() {
            total_time as f64 / images.len() as f64
        } else {
            0.0
        };

        println!("Total images: {}", images.len());
        println!("Total detections: {}", total_detections);
        println!("Total time: {}ms", total_time);
        println!("Average time: {:.2}ms", avg_time);

        (total_time, total_detections)
    }

    #[test]
    fn test_yolo11_vs_yolo26_benchmark() {
        init_tracing();

        println!("\n");
        println!("################################################");
        println!("# YOLO11 vs YOLO26 Inference Time Benchmark #");
        println!("################################################");

        let images = get_test_images();
        if images.is_empty() {
            println!("No test images found");
            return;
        }

        println!("Test images count: {}", images.len());

        let (time11, det11) = benchmark_model(MODEL11_PATH, "YOLO11", &images);
        let (time26, det26) = benchmark_model(MODEL26_PATH, "YOLO26", &images);

        println!("\n================================================");
        println!("=== Benchmark Comparison ===");
        println!("================================================");
        println!();

        if time11 > 0 && time26 > 0 {
            let avg11 = time11 as f64 / images.len() as f64;
            let avg26 = time26 as f64 / images.len() as f64;

            println!("| Metric        | YOLO11      | YOLO26      |");
            println!("|---------------|-------------|-------------|");
            println!("| Total time    | {}ms      | {}ms      |", time11, time26);
            println!("| Average time  | {:.2}ms    | {:.2}ms    |", avg11, avg26);
            println!("| Detections    | {}          | {}          |", det11, det26);
            println!("| Images        | {}          | {}          |", images.len(), images.len());
            println!();

            let speedup = if avg26 < avg11 {
                avg11 / avg26
            } else {
                avg26 / avg11
            };

            if avg26 < avg11 {
                println!("YOLO26 is {:.2}x faster than YOLO11", speedup);
            } else {
                println!("YOLO11 is {:.2}x faster than YOLO26", speedup);
            }

            let time_diff = (avg11 - avg26).abs();
            println!("Time difference: {:.2}ms per image", time_diff);
            println!();

            // Analysis
            println!("=== Analysis ===");
            println!("YOLO11 format: [1, 8, 8400] - requires NMS post-processing");
            println!("YOLO26 format: [1, 300, 6] - no NMS required (one-to-one head)");
            println!();

            // Estimate NMS overhead
            let nms_overhead_estimate = avg11 - avg26;
            if nms_overhead_estimate > 0.0 {
                println!("Estimated NMS overhead in YOLO11: {:.2}ms per image", nms_overhead_estimate);
            }
        } else {
            println!("Could not complete benchmark (models not found)");
        }
    }

    #[test]
    fn test_onnx_detection_with_custom_thresholds() {
        init_tracing();
        if !Path::new(MODEL11_PATH).exists() {
            eprintln!("Skipping test: Model not found at {}", MODEL11_PATH);
            return;
        }

        let images = get_test_images();
        if images.is_empty() {
            eprintln!("Skipping test: No test images found in {}", TEST_IMAGES_DIR);
            return;
        }

        let test_image = &images[0];

        let mut inference = OnnxInference::new(MODEL11_PATH, "test_thresholds");
        inference.initialize().expect("Failed to initialize model");

        let image = InferenceImage::from_file(test_image).expect("Failed to load image");

        // Test with high confidence threshold (fewer detections)
        let result_high = inference
            .detect_with_thresholds(&image, 0.5, 0.45)
            .expect("Detection with high threshold failed");

        // Test with low confidence threshold (more detections)
        let result_low = inference
            .detect_with_thresholds(&image, 0.1, 0.45)
            .expect("Detection with low threshold failed");

        println!(
            "High threshold (0.5): {} detections",
            result_high.detections.len()
        );
        println!(
            "Low threshold (0.1): {} detections",
            result_low.detections.len()
        );

        assert!(
            result_low.detections.len() >= result_high.detections.len(),
            "Lower threshold should produce more or equal detections"
        );
    }

    #[test]
    fn test_image_letterbox() {
        init_tracing();
        let images = get_test_images();
        if images.is_empty() {
            eprintln!("Skipping test: No test images found");
            return;
        }

        let image = InferenceImage::from_file(&images[0]).expect("Failed to load image");
        println!("Original image: {}x{}", image.width, image.height);

        // Test letterbox to 640x640
        let (letterboxed, scale, pad_x, pad_y) = image.letterbox(640, 640, (114, 114, 114));

        println!("Letterboxed: {}x{}", letterboxed.width, letterboxed.height);
        println!("Scale: {}, Pad: ({}, {})", scale, pad_x, pad_y);

        assert_eq!(letterboxed.width, 640);
        assert_eq!(letterboxed.height, 640);
        assert_eq!(letterboxed.channels, 3);
    }

    #[test]
    fn test_image_resize() {
        init_tracing();
        let images = get_test_images();
        if images.is_empty() {
            eprintln!("Skipping test: No test images found");
            return;
        }

        let image = InferenceImage::from_file(&images[0]).expect("Failed to load image");

        // Test resize
        let resized = image.resize(320, 320);
        assert_eq!(resized.width, 320);
        assert_eq!(resized.height, 320);
        assert_eq!(resized.channels, 3);
    }
}