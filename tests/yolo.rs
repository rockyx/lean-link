#[cfg(feature = "inspection")]
pub mod test {
    use std::path::Path;

    use lean_link::service::inspection::detector::Detector;
    use lean_link::service::inspection::image::InferenceImage;
    use lean_link::service::inspection::yolo::OnnxInference;

    const MODEL_PATH: &str = r"C:\Users\fz_ka\Development\yolo\runs\detect\line1\weights\best.onnx";
    const TEST_IMAGES_DIR: &str = r"C:\Users\fz_ka\Development\yolo\line1\val\images";

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

    #[test]
    fn test_onnx_model_initialization() {
        if !Path::new(MODEL_PATH).exists() {
            eprintln!("Skipping test: Model not found at {}", MODEL_PATH);
            return;
        }

        let mut inference = OnnxInference::new(MODEL_PATH, "test_model");
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
        if !Path::new(MODEL_PATH).exists() {
            eprintln!("Skipping test: Model not found at {}", MODEL_PATH);
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
        let mut inference = OnnxInference::new(MODEL_PATH, "test_detection");
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
        tracing_subscriber::fmt()
            .with_max_level(tracing::Level::DEBUG)
            .init();
        if !Path::new(MODEL_PATH).exists() {
            eprintln!("Skipping test: Model not found at {}", MODEL_PATH);
            return;
        }

        let images = get_test_images();
        if images.is_empty() {
            eprintln!("Skipping test: No test images found in {}", TEST_IMAGES_DIR);
            return;
        }

        // Initialize model once
        let mut inference = OnnxInference::new(MODEL_PATH, "test_batch");
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

    #[test]
    fn test_onnx_detection_with_custom_thresholds() {
        tracing_subscriber::fmt()
            .with_max_level(tracing::Level::DEBUG)
            .init();
        if !Path::new(MODEL_PATH).exists() {
            eprintln!("Skipping test: Model not found at {}", MODEL_PATH);
            return;
        }

        let images = get_test_images();
        if images.is_empty() {
            eprintln!("Skipping test: No test images found in {}", TEST_IMAGES_DIR);
            return;
        }

        let test_image = &images[0];

        let mut inference = OnnxInference::new(MODEL_PATH, "test_thresholds");
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
        tracing_subscriber::fmt()
            .with_max_level(tracing::Level::DEBUG)
            .init();
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
        tracing_subscriber::fmt()
            .with_max_level(tracing::Level::DEBUG)
            .init();
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
