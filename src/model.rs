use serde_json::Value;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Service {
    Ec2,
    Ecs,
    Ecr,
    S3,
}

impl Service {
    pub const ALL: [Self; 4] = [Self::Ec2, Self::Ecs, Self::Ecr, Self::S3];

    pub fn name(self) -> &'static str {
        match self {
            Self::Ec2 => "EC2",
            Self::Ecs => "ECS",
            Self::Ecr => "ECR",
            Self::S3 => "S3",
        }
    }

    pub fn index(self) -> usize {
        Self::ALL.iter().position(|service| *service == self).unwrap_or(0)
    }

    pub fn panel_titles(self) -> [&'static str; 4] {
        match self {
            Self::Ec2 => ["Instances", "Volumes", "Security groups", "Tags"],
            Self::Ecs => ["Clusters", "Services", "Tasks", "Task definitions"],
            Self::Ecr => ["Repositories", "Images", "Image tags", "Repository tags"],
            Self::S3 => ["Buckets", "Objects", "Prefixes", "Bucket settings"],
        }
    }
}

#[derive(Clone, Debug)]
pub struct Resource {
    pub id: String,
    pub label: String,
    pub summary: String,
    pub detail: Value,
}

#[derive(Clone, Debug)]
pub struct Panel {
    pub title: String,
    pub items: Vec<Resource>,
    pub error: Option<String>,
}

#[derive(Clone, Debug)]
pub struct Snapshot {
    pub service: Service,
    pub panels: [Panel; 4],
    pub logs: Vec<String>,
}

impl Snapshot {
    pub fn empty(service: Service) -> Self {
        Self {
            service,
            panels: service.panel_titles().map(|title| Panel {
                title: title.into(),
                items: Vec::new(),
                error: None,
            }),
            logs: Vec::new(),
        }
    }
}
