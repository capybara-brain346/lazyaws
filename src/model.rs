use serde_json::Value;

pub const PANEL_COUNT: usize = 4;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Service {
    Ec2,
    Ecs,
    Ecr,
    S3,
    Lambda,
    Cost,
}

impl Service {
    pub const ALL: [Self; 6] = [
        Self::Ec2,
        Self::Ecs,
        Self::Ecr,
        Self::S3,
        Self::Lambda,
        Self::Cost,
    ];

    pub fn name(self) -> &'static str {
        match self {
            Self::Ec2 => "EC2",
            Self::Ecs => "ECS",
            Self::Ecr => "ECR",
            Self::S3 => "S3",
            Self::Lambda => "Lambda",
            Self::Cost => "Cost",
        }
    }

    // Cost Explorer charges for each request, so the Cost tab loads only on confirmation.
    pub fn auto_refresh(self) -> bool {
        self != Self::Cost
    }

    pub fn index(self) -> usize {
        Self::ALL
            .iter()
            .position(|service| *service == self)
            .unwrap_or(0)
    }

    pub fn next(self) -> Self {
        Self::ALL[(self.index() + 1) % Self::ALL.len()]
    }

    pub fn previous(self) -> Self {
        Self::ALL[(self.index() + Self::ALL.len() - 1) % Self::ALL.len()]
    }

    pub fn panel_titles(self) -> [&'static str; PANEL_COUNT] {
        match self {
            Self::Ec2 => ["Instances", "Volumes", "Security groups", "Tags"],
            Self::Ecs => ["Clusters", "Services", "Tasks", "Task definitions"],
            Self::Ecr => ["Repositories", "Images", "Image tags", "Repository tags"],
            Self::S3 => ["Buckets", "Objects", "Prefixes", "Bucket settings"],
            Self::Lambda => ["Functions", "Versions", "Aliases", "Tags"],
            Self::Cost => ["Summary", "By service", "Daily", "By region"],
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

#[derive(Clone, Debug, Default)]
pub struct Panel {
    pub title: String,
    pub items: Vec<Resource>,
    pub error: Option<String>,
}

#[derive(Clone, Debug)]
pub struct Snapshot {
    pub service: Service,
    pub panels: [Panel; PANEL_COUNT],
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
