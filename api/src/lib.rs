pub mod handlers {
    #[path = "report_handler.rs"]
    pub mod report_handler;

    #[path = "manifest.rs"]
    pub mod manifest;
}

pub mod models {
    #[path = "report.rs"]
    pub mod report;
}

pub mod routes {
    #[path = "report_route.rs"]
    pub mod report_route;
}

pub mod services {
    #[path = "report_service.rs"]
    pub mod report_service;
}

pub mod helpers {
    #[path = "allure_config.rs"]
    pub mod allure_config;

    #[path = "fs_helper.rs"]
    pub mod fs_helper;

    #[path = "zip_helper.rs"]
    pub mod zip_helper;

    #[path = "allure_generator.rs"]
    pub mod allure_generator;

    #[path = "access_control.rs"]
    pub mod access_control;
}

pub mod route;
