//! Fixture generation utilities for creating test projects
//!
//! This module provides tools for generating realistic test fixtures
//! with parameterized complexity, dependencies, and structure.

#![allow(clippy::uninlined_format_args)]
#![allow(dead_code)]

use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

/// Builder for generating test project fixtures
///
/// Creates realistic task files with proper structure, dependencies,
/// and labels for testing.
pub struct ProjectGenerator {
    /// Project name/ID
    name: String,
    /// Project title
    title: String,
    /// Files to generate (path -> content)
    files: Vec<(PathBuf, FileContent)>,
    /// Project labels
    labels: Vec<String>,
    /// Base creation date
    base_date: String,
}

/// Content for a generated file
struct FileContent {
    /// File ID
    id: String,
    /// File title
    title: String,
    /// File description
    description: Option<String>,
    /// File labels
    labels: Vec<String>,
    /// Tasks to include
    tasks: Vec<TaskContent>,
    /// Dependencies on other files
    depends_on: Vec<String>,
}

/// Content for a generated task
struct TaskContent {
    /// Task text
    text: String,
    /// Task status: ' ' (open), 'x' (done), '-' (waived)
    status: char,
    /// Task labels
    labels: Vec<String>,
    /// Nested subtasks
    subtasks: Vec<TaskContent>,
    /// Optional ID for referencing
    id: Option<String>,
}

impl ProjectGenerator {
    /// Create a new project generator
    pub fn new(name: impl Into<String>, title: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            title: title.into(),
            files: Vec::new(),
            labels: Vec::new(),
            base_date: "2024-01-10".to_string(),
        }
    }

    /// Add project-level labels
    pub fn with_labels(mut self, labels: Vec<String>) -> Self {
        self.labels = labels;
        self
    }

    /// Set base date for tasks
    pub fn with_base_date(mut self, date: impl Into<String>) -> Self {
        self.base_date = date.into();
        self
    }

    /// Add a file to the project
    pub fn add_file(
        self,
        path: impl Into<PathBuf>,
        id: impl Into<String>,
        title: impl Into<String>,
    ) -> FileBuilder {
        FileBuilder {
            generator: self,
            path: path.into(),
            id: id.into(),
            title: title.into(),
            description: None,
            labels: Vec::new(),
            tasks: Vec::new(),
            depends_on: Vec::new(),
        }
    }

    /// Generate all files to a directory
    ///
    /// # Errors
    ///
    /// Returns error if files cannot be written
    pub fn generate_to(&self, output_dir: &Path) -> std::io::Result<()> {
        // Create output directory
        fs::create_dir_all(output_dir)?;

        // Generate index file
        let index_content = self.generate_index();
        let index_path = output_dir.join("lash.index.md");
        fs::write(index_path, index_content)?;

        // Generate all other files
        for (path, content) in &self.files {
            let full_path = output_dir.join(path);

            // Create parent directories
            if let Some(parent) = full_path.parent() {
                fs::create_dir_all(parent)?;
            }

            let file_content = content.generate();
            fs::write(full_path, file_content)?;
        }

        Ok(())
    }

    /// Generate the index file content
    fn generate_index(&self) -> String {
        let mut content = String::new();

        // Header
        content.push_str(&format!("# {}\n\n", self.title));

        // Metadata
        content.push_str(&format!("@id: {}\n", self.name));
        if !self.labels.is_empty() {
            content.push_str(&format!("@labels: {}\n", self.labels.join(", ")));
        }
        content.push_str(&format!("@created: {}\n\n", self.base_date));

        // Description
        content.push_str(&format!(
            "Test project: {} with {} files demonstrating realistic task structure and dependencies.\n\n",
            self.title,
            self.files.len()
        ));

        // Structure overview
        content.push_str("## Structure\n\n");
        let mut dirs: HashMap<String, Vec<&PathBuf>> = HashMap::new();
        for (path, _) in &self.files {
            if let Some(parent) = path.parent() {
                let parent_str = parent.to_string_lossy().to_string();
                dirs.entry(parent_str).or_default().push(path);
            }
        }

        for (dir, paths) in dirs.iter() {
            if !dir.is_empty() {
                content.push_str(&format!("- `{}/` - Module tasks\n", dir));
                for path in paths {
                    content.push_str(&format!("  - `{}`\n", path.display()));
                }
            }
        }

        content.push_str("\n## Tasks\n\n");
        content.push_str("- [ ] Project setup\n");
        content.push_str("- [ ] Core implementation\n");
        content.push_str("- [ ] Testing and QA\n");
        content.push_str("- [ ] Documentation\n");

        content
    }
}

/// Builder for adding a file to a project
pub struct FileBuilder {
    generator: ProjectGenerator,
    path: PathBuf,
    id: String,
    title: String,
    description: Option<String>,
    labels: Vec<String>,
    tasks: Vec<TaskContent>,
    depends_on: Vec<String>,
}

impl FileBuilder {
    /// Add a description to the file
    pub fn with_description(mut self, description: impl Into<String>) -> Self {
        self.description = Some(description.into());
        self
    }

    /// Add labels to the file
    pub fn with_labels(mut self, labels: Vec<String>) -> Self {
        self.labels = labels;
        self
    }

    /// Add a dependency on another file
    pub fn depends_on(mut self, file_id: impl Into<String>) -> Self {
        self.depends_on.push(file_id.into());
        self
    }

    /// Add a task to the file
    pub fn add_task(self, text: impl Into<String>) -> TaskBuilder {
        TaskBuilder {
            file_builder: self,
            text: text.into(),
            status: ' ',
            labels: Vec::new(),
            subtasks: Vec::new(),
            id: None,
        }
    }

    /// Finish building this file and return to project builder
    pub fn done(mut self) -> ProjectGenerator {
        let content = FileContent {
            id: self.id,
            title: self.title,
            description: self.description,
            labels: self.labels,
            tasks: self.tasks,
            depends_on: self.depends_on,
        };
        self.generator.files.push((self.path, content));
        self.generator
    }
}

/// Builder for adding a task to a file
pub struct TaskBuilder {
    file_builder: FileBuilder,
    text: String,
    status: char,
    labels: Vec<String>,
    subtasks: Vec<TaskContent>,
    id: Option<String>,
}

impl TaskBuilder {
    /// Set task status
    pub fn with_status(mut self, status: char) -> Self {
        self.status = status;
        self
    }

    /// Mark task as done
    pub fn done(mut self) -> Self {
        self.status = 'x';
        self
    }

    /// Mark task as waived
    pub fn waived(mut self) -> Self {
        self.status = '-';
        self
    }

    /// Add labels to the task
    pub fn with_labels(mut self, labels: Vec<String>) -> Self {
        self.labels = labels;
        self
    }

    /// Add an ID to the task
    pub fn with_id(mut self, id: impl Into<String>) -> Self {
        self.id = Some(id.into());
        self
    }

    /// Add a subtask
    pub fn add_subtask(
        mut self,
        text: impl Into<String>,
        status: char,
        labels: Vec<String>,
    ) -> Self {
        self.subtasks.push(TaskContent {
            text: text.into(),
            status,
            labels,
            subtasks: Vec::new(),
            id: None,
        });
        self
    }

    /// Finish building this task and return to file builder
    pub fn end_task(mut self) -> FileBuilder {
        self.file_builder.tasks.push(TaskContent {
            text: self.text,
            status: self.status,
            labels: self.labels,
            subtasks: self.subtasks,
            id: self.id,
        });
        self.file_builder
    }
}

impl FileContent {
    /// Generate the markdown content for this file
    fn generate(&self) -> String {
        let mut content = String::new();

        // Header
        content.push_str(&format!("# {}\n\n", self.title));

        // Metadata
        content.push_str(&format!("@id: {}\n", self.id));
        if !self.labels.is_empty() {
            content.push_str(&format!("@labels: {}\n", self.labels.join(", ")));
        }
        if !self.depends_on.is_empty() {
            for dep in &self.depends_on {
                content.push_str(&format!("@depends-on: {}\n", dep));
            }
        }
        content.push_str("@created: 2024-01-15\n\n");

        // Description
        if let Some(desc) = &self.description {
            content.push_str(desc);
            content.push_str("\n\n");
        }

        // Tasks
        content.push_str("## Tasks\n\n");
        for task in &self.tasks {
            task.generate_to(&mut content, 0);
        }

        content
    }
}

impl TaskContent {
    /// Generate task markdown with proper indentation
    fn generate_to(&self, output: &mut String, indent_level: usize) {
        let indent = "  ".repeat(indent_level);

        // Task checkbox and text with proper indentation
        output.push_str(&indent);
        output.push_str(&format!("- [{}] {}", self.status, self.text));

        // Add labels inline
        if !self.labels.is_empty() {
            output.push(' ');
            for label in &self.labels {
                output.push_str(&format!("#{} ", label));
            }
            output.pop(); // Remove trailing space
        }

        // Add ID annotation if present
        if let Some(id) = &self.id {
            output.push_str(&format!(" @id: {}", id));
        }

        output.push('\n');

        // Subtasks
        for subtask in &self.subtasks {
            subtask.generate_to(output, indent_level + 1);
        }
    }
}

/// Generate a realistic e-commerce project fixture
///
/// Creates a 75-file project with ~300 tasks demonstrating:
/// - Realistic directory structure (backend, frontend, mobile, infrastructure, docs)
/// - Cross-file dependencies
/// - Various task statuses
/// - Comprehensive label usage
///
/// # Errors
///
/// Returns error if files cannot be written
pub fn generate_ecommerce_project(output_dir: &Path) -> std::io::Result<()> {
    let generator = ProjectGenerator::new("ecommerce-platform", "E-Commerce Platform")
        .with_labels(vec!["fullstack".into(), "production".into()])
        .with_base_date("2024-01-01");

    // We'll build this incrementally by adding modules
    let gen = add_backend_module(generator);
    let gen = add_frontend_module(gen);
    let gen = add_mobile_module(gen);
    let gen = add_infrastructure_module(gen);
    let gen = add_docs_module(gen);

    gen.generate_to(output_dir)
}

/// Add backend module files to generator
fn add_backend_module(gen: ProjectGenerator) -> ProjectGenerator {
    gen.add_file(
        "backend/authentication.md",
        "backend-auth",
        "Authentication Service",
    )
    .with_labels(vec!["backend".into(), "security".into()])
    .with_description("User authentication and authorization system.")
    .add_task("Implement JWT token generation")
    .with_labels(vec!["p0".into()])
    .add_subtask("Create token signing logic", 'x', vec![])
    .add_subtask("Add token validation", 'x', vec![])
    .add_subtask("Implement refresh token flow", ' ', vec!["p1".into()])
    .end_task()
    .add_task("Implement OAuth2 providers")
    .add_subtask("Google OAuth", 'x', vec![])
    .add_subtask("GitHub OAuth", ' ', vec![])
    .add_subtask("Apple OAuth", ' ', vec![])
    .end_task()
    .add_task("Add password reset flow")
    .with_status(' ')
    .end_task()
    .add_task("Implement 2FA support")
    .add_subtask("TOTP generation", ' ', vec![])
    .add_subtask("SMS backup codes", ' ', vec![])
    .end_task()
    .done()
    .add_file("backend/api-gateway.md", "backend-gateway", "API Gateway")
    .with_labels(vec!["backend".into(), "infrastructure".into()])
    .depends_on("backend/authentication.md")
    .add_task("Set up route configuration")
    .done()
    .end_task()
    .add_task("Implement rate limiting")
    .add_subtask("Per-IP rate limits", 'x', vec![])
    .add_subtask("Per-user rate limits", ' ', vec![])
    .add_subtask("Rate limit headers", ' ', vec![])
    .end_task()
    .add_task("Add request logging and metrics")
    .end_task()
    .add_task("Implement circuit breaker pattern")
    .end_task()
    .done()
    .add_file(
        "backend/product-catalog.md",
        "backend-products",
        "Product Catalog Service",
    )
    .with_labels(vec!["backend".into(), "feature".into()])
    .add_task("Design product schema")
    .done()
    .end_task()
    .add_task("Implement CRUD endpoints")
    .add_subtask("Create product", 'x', vec![])
    .add_subtask("Update product", 'x', vec![])
    .add_subtask("Delete product", 'x', vec![])
    .add_subtask("Get product by ID", 'x', vec![])
    .add_subtask("List products with pagination", ' ', vec![])
    .end_task()
    .add_task("Add product search")
    .add_subtask("Full-text search", ' ', vec!["p0".into()])
    .add_subtask("Faceted filtering", ' ', vec!["p1".into()])
    .add_subtask("Search relevance tuning", ' ', vec!["p2".into()])
    .end_task()
    .add_task("Implement inventory management")
    .add_subtask("Stock tracking", ' ', vec![])
    .add_subtask("Low stock alerts", ' ', vec![])
    .end_task()
    .done()
    .add_file(
        "backend/order-processing.md",
        "backend-orders",
        "Order Processing Service",
    )
    .with_labels(vec!["backend".into(), "feature".into()])
    .depends_on("backend/product-catalog.md")
    .add_task("Implement order creation")
    .add_subtask("Validate cart items", 'x', vec![])
    .add_subtask("Calculate totals", 'x', vec![])
    .add_subtask("Reserve inventory", ' ', vec![])
    .add_subtask("Create order record", ' ', vec![])
    .end_task()
    .add_task("Add payment integration")
    .add_subtask("Stripe integration", ' ', vec!["p0".into()])
    .add_subtask("PayPal integration", ' ', vec!["p1".into()])
    .add_subtask("Apple Pay support", ' ', vec!["p2".into()])
    .end_task()
    .add_task("Implement order status tracking")
    .add_subtask("Status transitions", ' ', vec![])
    .add_subtask("Email notifications", ' ', vec![])
    .add_subtask("Webhook callbacks", ' ', vec![])
    .end_task()
    .done()
    .add_file("backend/database.md", "backend-db", "Database Layer")
    .with_labels(vec!["backend".into(), "infrastructure".into()])
    .add_task("Design database schema")
    .done()
    .end_task()
    .add_task("Set up migrations")
    .done()
    .end_task()
    .add_task("Implement connection pooling")
    .done()
    .end_task()
    .add_task("Add database indexes")
    .add_subtask("Product search indexes", 'x', vec![])
    .add_subtask("Order query indexes", ' ', vec![])
    .add_subtask("User lookup indexes", 'x', vec![])
    .end_task()
    .add_task("Configure read replicas")
    .end_task()
    .done()
    .add_file("backend/tests.md", "backend-tests", "Backend Testing")
    .with_labels(vec!["backend".into(), "testing".into()])
    .add_task("Unit tests")
    .add_subtask("Authentication tests", 'x', vec![])
    .add_subtask("Product service tests", 'x', vec![])
    .add_subtask("Order service tests", ' ', vec![])
    .end_task()
    .add_task("Integration tests")
    .add_subtask("API endpoint tests", ' ', vec![])
    .add_subtask("Database integration tests", ' ', vec![])
    .end_task()
    .add_task("Load testing")
    .add_subtask("Product API load tests", ' ', vec![])
    .add_subtask("Order processing load tests", ' ', vec![])
    .end_task()
    .done()
}

/// Add frontend module files to generator
fn add_frontend_module(gen: ProjectGenerator) -> ProjectGenerator {
    gen.add_file(
        "frontend/components.md",
        "frontend-components",
        "UI Components",
    )
    .with_labels(vec!["frontend".into(), "ui".into()])
    .add_task("Product listing components")
    .add_subtask("ProductCard component", 'x', vec![])
    .add_subtask("ProductGrid component", 'x', vec![])
    .add_subtask("ProductFilters component", ' ', vec![])
    .add_subtask("Pagination component", ' ', vec![])
    .end_task()
    .add_task("Shopping cart components")
    .add_subtask("CartItem component", 'x', vec![])
    .add_subtask("CartSummary component", ' ', vec![])
    .add_subtask("CartDrawer component", ' ', vec![])
    .end_task()
    .add_task("Checkout components")
    .add_subtask("CheckoutForm component", ' ', vec![])
    .add_subtask("PaymentMethod component", ' ', vec![])
    .add_subtask("ShippingAddress component", ' ', vec![])
    .add_subtask("OrderSummary component", ' ', vec![])
    .end_task()
    .add_task("User account components")
    .add_subtask("ProfileForm component", ' ', vec![])
    .add_subtask("OrderHistory component", ' ', vec![])
    .add_subtask("WishList component", ' ', vec!["p2".into()])
    .end_task()
    .done()
    .add_file(
        "frontend/state-management.md",
        "frontend-state",
        "State Management",
    )
    .with_labels(vec!["frontend".into()])
    .depends_on("frontend/components.md")
    .add_task("Set up state management library")
    .done()
    .end_task()
    .add_task("Implement cart state")
    .add_subtask("Add to cart action", 'x', vec![])
    .add_subtask("Remove from cart action", 'x', vec![])
    .add_subtask("Update quantity action", 'x', vec![])
    .add_subtask("Clear cart action", ' ', vec![])
    .end_task()
    .add_task("Implement user state")
    .add_subtask("Login action", 'x', vec![])
    .add_subtask("Logout action", 'x', vec![])
    .add_subtask("Update profile action", ' ', vec![])
    .end_task()
    .add_task("Implement product state")
    .add_subtask("Fetch products action", ' ', vec![])
    .add_subtask("Search products action", ' ', vec![])
    .add_subtask("Filter products action", ' ', vec![])
    .end_task()
    .done()
    .add_file(
        "frontend/routing.md",
        "frontend-routing",
        "Application Routing",
    )
    .with_labels(vec!["frontend".into()])
    .add_task("Set up route configuration")
    .done()
    .end_task()
    .add_task("Implement route guards")
    .add_subtask("Authentication guard", 'x', vec![])
    .add_subtask("Admin guard", ' ', vec![])
    .end_task()
    .add_task("Add lazy loading for routes")
    .end_task()
    .done()
    .add_file("frontend/api-client.md", "frontend-api", "API Client")
    .with_labels(vec!["frontend".into()])
    .depends_on("backend/api-gateway.md")
    .add_task("Set up HTTP client")
    .done()
    .end_task()
    .add_task("Implement authentication interceptor")
    .done()
    .end_task()
    .add_task("Add error handling")
    .add_subtask("Network error handling", 'x', vec![])
    .add_subtask("Validation error display", ' ', vec![])
    .add_subtask("Retry logic", ' ', vec![])
    .end_task()
    .add_task("Implement API methods")
    .add_subtask("Product API methods", ' ', vec![])
    .add_subtask("Order API methods", ' ', vec![])
    .add_subtask("User API methods", ' ', vec![])
    .end_task()
    .done()
    .add_file(
        "frontend/styling.md",
        "frontend-styles",
        "Styling and Theming",
    )
    .with_labels(vec!["frontend".into(), "ui".into()])
    .add_task("Set up design system")
    .done()
    .end_task()
    .add_task("Implement theme variables")
    .add_subtask("Color palette", 'x', vec![])
    .add_subtask("Typography scale", 'x', vec![])
    .add_subtask("Spacing system", 'x', vec![])
    .end_task()
    .add_task("Add dark mode support")
    .add_subtask("Dark theme colors", ' ', vec![])
    .add_subtask("Theme toggle component", ' ', vec![])
    .add_subtask("Persist theme preference", ' ', vec![])
    .end_task()
    .add_task("Implement responsive design")
    .add_subtask("Mobile breakpoints", ' ', vec!["p0".into()])
    .add_subtask("Tablet breakpoints", ' ', vec!["p1".into()])
    .add_subtask("Desktop optimization", ' ', vec![])
    .end_task()
    .done()
    .add_file("frontend/tests.md", "frontend-tests", "Frontend Testing")
    .with_labels(vec!["frontend".into(), "testing".into()])
    .add_task("Component unit tests")
    .add_subtask("ProductCard tests", 'x', vec![])
    .add_subtask("CartItem tests", 'x', vec![])
    .add_subtask("CheckoutForm tests", ' ', vec![])
    .end_task()
    .add_task("Integration tests")
    .add_subtask("Shopping flow test", ' ', vec![])
    .add_subtask("Checkout flow test", ' ', vec![])
    .end_task()
    .add_task("E2E tests")
    .add_subtask("Purchase flow E2E", ' ', vec![])
    .add_subtask("User registration E2E", ' ', vec![])
    .end_task()
    .done()
}

/// Add mobile module files to generator
fn add_mobile_module(gen: ProjectGenerator) -> ProjectGenerator {
    gen.add_file("mobile/ios-app.md", "mobile-ios", "iOS Application")
        .with_labels(vec!["mobile".into(), "ios".into()])
        .add_task("Set up iOS project")
        .done()
        .end_task()
        .add_task("Implement core screens")
        .add_subtask("Home screen", 'x', vec![])
        .add_subtask("Product detail screen", ' ', vec![])
        .add_subtask("Cart screen", ' ', vec![])
        .add_subtask("Checkout screen", ' ', vec![])
        .add_subtask("Profile screen", ' ', vec![])
        .end_task()
        .add_task("Add iOS-specific features")
        .add_subtask("Face ID authentication", ' ', vec![])
        .add_subtask("Apple Pay integration", ' ', vec![])
        .add_subtask("Push notifications", ' ', vec![])
        .end_task()
        .add_task("App Store submission")
        .add_subtask("Screenshots", ' ', vec![])
        .add_subtask("App description", ' ', vec![])
        .add_subtask("Privacy policy", ' ', vec![])
        .end_task()
        .done()
        .add_file(
            "mobile/android-app.md",
            "mobile-android",
            "Android Application",
        )
        .with_labels(vec!["mobile".into(), "android".into()])
        .add_task("Set up Android project")
        .done()
        .end_task()
        .add_task("Implement core screens")
        .add_subtask("Home screen", 'x', vec![])
        .add_subtask("Product detail screen", ' ', vec![])
        .add_subtask("Cart screen", ' ', vec![])
        .add_subtask("Checkout screen", ' ', vec![])
        .add_subtask("Profile screen", ' ', vec![])
        .end_task()
        .add_task("Add Android-specific features")
        .add_subtask("Fingerprint authentication", ' ', vec![])
        .add_subtask("Google Pay integration", ' ', vec![])
        .add_subtask("FCM notifications", ' ', vec![])
        .end_task()
        .add_task("Play Store submission")
        .add_subtask("Screenshots", ' ', vec![])
        .add_subtask("App description", ' ', vec![])
        .add_subtask("Privacy policy", ' ', vec![])
        .end_task()
        .done()
        .add_file(
            "mobile/shared-logic.md",
            "mobile-shared",
            "Shared Mobile Logic",
        )
        .with_labels(vec!["mobile".into()])
        .add_task("Implement shared business logic")
        .add_subtask("API client", 'x', vec![])
        .add_subtask("Data models", 'x', vec![])
        .add_subtask("Validation logic", ' ', vec![])
        .end_task()
        .add_task("Add offline support")
        .add_subtask("Local caching", ' ', vec![])
        .add_subtask("Sync logic", ' ', vec![])
        .end_task()
        .done()
}

/// Add infrastructure module files to generator
fn add_infrastructure_module(gen: ProjectGenerator) -> ProjectGenerator {
    gen.add_file("infrastructure/ci-cd.md", "infra-cicd", "CI/CD Pipeline")
        .with_labels(vec!["infrastructure".into(), "devops".into()])
        .add_task("Set up CI pipeline")
        .add_subtask("Configure build jobs", 'x', vec![])
        .add_subtask("Add test jobs", 'x', vec![])
        .add_subtask("Add lint jobs", 'x', vec![])
        .end_task()
        .add_task("Set up CD pipeline")
        .add_subtask("Staging deployment", 'x', vec![])
        .add_subtask("Production deployment", ' ', vec![])
        .add_subtask("Rollback strategy", ' ', vec![])
        .end_task()
        .add_task("Add deployment automation")
        .add_subtask("Database migrations", ' ', vec![])
        .add_subtask("Asset deployment", ' ', vec![])
        .add_subtask("Cache invalidation", ' ', vec![])
        .end_task()
        .done()
        .add_file(
            "infrastructure/monitoring.md",
            "infra-monitoring",
            "Monitoring and Observability",
        )
        .with_labels(vec!["infrastructure".into(), "observability".into()])
        .add_task("Set up application monitoring")
        .add_subtask("Error tracking", 'x', vec![])
        .add_subtask("Performance monitoring", ' ', vec![])
        .add_subtask("User analytics", ' ', vec![])
        .end_task()
        .add_task("Add logging infrastructure")
        .add_subtask("Centralized logging", ' ', vec![])
        .add_subtask("Log aggregation", ' ', vec![])
        .add_subtask("Log search and alerts", ' ', vec![])
        .end_task()
        .add_task("Implement alerting")
        .add_subtask("Error rate alerts", ' ', vec![])
        .add_subtask("Performance alerts", ' ', vec![])
        .add_subtask("Uptime monitoring", ' ', vec![])
        .end_task()
        .done()
        .add_file(
            "infrastructure/hosting.md",
            "infra-hosting",
            "Hosting and Infrastructure",
        )
        .with_labels(vec!["infrastructure".into(), "cloud".into()])
        .add_task("Set up cloud infrastructure")
        .add_subtask("VPC configuration", 'x', vec![])
        .add_subtask("Load balancers", 'x', vec![])
        .add_subtask("Auto-scaling groups", ' ', vec![])
        .end_task()
        .add_task("Configure databases")
        .add_subtask("Primary database", 'x', vec![])
        .add_subtask("Read replicas", ' ', vec![])
        .add_subtask("Backup strategy", ' ', vec![])
        .end_task()
        .add_task("Set up CDN")
        .add_subtask("Static asset delivery", ' ', vec![])
        .add_subtask("Image optimization", ' ', vec![])
        .add_subtask("Cache configuration", ' ', vec![])
        .end_task()
        .done()
        .add_file(
            "infrastructure/security.md",
            "infra-security",
            "Security and Compliance",
        )
        .with_labels(vec!["infrastructure".into(), "security".into()])
        .add_task("Implement security measures")
        .add_subtask("SSL/TLS certificates", 'x', vec![])
        .add_subtask("WAF configuration", ' ', vec![])
        .add_subtask("DDoS protection", ' ', vec![])
        .end_task()
        .add_task("Add security scanning")
        .add_subtask("Dependency scanning", ' ', vec![])
        .add_subtask("Code security scanning", ' ', vec![])
        .add_subtask("Container scanning", ' ', vec![])
        .end_task()
        .add_task("Compliance requirements")
        .add_subtask("GDPR compliance", ' ', vec![])
        .add_subtask("PCI DSS compliance", ' ', vec!["p0".into()])
        .add_subtask("SOC 2 audit", ' ', vec!["p1".into()])
        .end_task()
        .done()
}

/// Add documentation module files to generator
fn add_docs_module(gen: ProjectGenerator) -> ProjectGenerator {
    gen.add_file("docs/api-documentation.md", "docs-api", "API Documentation")
        .with_labels(vec!["docs".into(), "backend".into()])
        .add_task("Document API endpoints")
        .add_subtask("Authentication endpoints", 'x', vec![])
        .add_subtask("Product endpoints", ' ', vec![])
        .add_subtask("Order endpoints", ' ', vec![])
        .add_subtask("User endpoints", ' ', vec![])
        .end_task()
        .add_task("Add API examples")
        .add_subtask("cURL examples", ' ', vec![])
        .add_subtask("SDK examples", ' ', vec![])
        .end_task()
        .add_task("Generate OpenAPI spec")
        .end_task()
        .done()
        .add_file("docs/user-guide.md", "docs-user", "User Guide")
        .with_labels(vec!["docs".into(), "user-facing".into()])
        .add_task("Write getting started guide")
        .add_subtask("Account creation", ' ', vec![])
        .add_subtask("First purchase", ' ', vec![])
        .add_subtask("Profile setup", ' ', vec![])
        .end_task()
        .add_task("Document core features")
        .add_subtask("Product search", ' ', vec![])
        .add_subtask("Shopping cart", ' ', vec![])
        .add_subtask("Checkout process", ' ', vec![])
        .add_subtask("Order tracking", ' ', vec![])
        .end_task()
        .add_task("Add troubleshooting section")
        .add_subtask("Common issues", ' ', vec![])
        .add_subtask("FAQ", ' ', vec![])
        .end_task()
        .done()
        .add_file("docs/developer-guide.md", "docs-dev", "Developer Guide")
        .with_labels(vec!["docs".into(), "internal".into()])
        .add_task("Document development setup")
        .add_subtask("Local environment setup", 'x', vec![])
        .add_subtask("Database setup", 'x', vec![])
        .add_subtask("Running tests", 'x', vec![])
        .end_task()
        .add_task("Document architecture")
        .add_subtask("System overview", ' ', vec![])
        .add_subtask("Database schema", ' ', vec![])
        .add_subtask("API design", ' ', vec![])
        .end_task()
        .add_task("Add deployment guide")
        .add_subtask("Staging deployment", ' ', vec![])
        .add_subtask("Production deployment", ' ', vec![])
        .add_subtask("Rollback procedures", ' ', vec![])
        .end_task()
        .done()
}

// Export pixelquest module
pub mod pixelquest;
