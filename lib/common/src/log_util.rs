pub fn init_tracing() {
    eprintln!(
        r#"    ____            __ _                          
   / __/____ _ ____/ /(_)____ _        _____ _____
  / /_ / __ `// __  // // __ `/______ / ___// ___/
 / __// /_/ // /_/ // // /_/ //_____// /   (__  ) 
/_/   \__,_/ \__,_//_/ \__,_/       /_/   /____/  
                                                  "#
    );

    tracing_subscriber::fmt()
        .without_time()
        .with_target(false)
        .with_max_level(tracing::Level::DEBUG)
        .init();
}
