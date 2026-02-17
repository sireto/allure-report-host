pub mod handlers {
    #[path = "report_handler.rs"]
    pub mod report_handler;
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
}

pub mod route;