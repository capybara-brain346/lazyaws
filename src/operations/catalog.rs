use serde_json::Value;

use crate::model::{Resource, Service, Snapshot};

use super::{InvokeMode, Operation, Parameter};

mod ec2;
mod ecr;
mod ecs;
mod lambda;
mod s3;

#[cfg(test)]
mod tests;

pub fn available(
    service: Service,
    panel: usize,
    resource: &Resource,
    primary: &Resource,
    snapshot: &Snapshot,
) -> Vec<Operation> {
    match service {
        Service::Ec2 => ec2::available(panel, resource, primary),
        Service::Ecs => ecs::available(panel, resource, primary),
        Service::Ecr => ecr::available(panel, resource, primary),
        Service::S3 => s3::available(panel, resource, primary, snapshot),
        Service::Lambda => lambda::available(panel, resource, primary),
        Service::Cost => Vec::new(),
    }
}

pub(super) fn operation(
    label: &'static str,
    service: &'static str,
    name: &'static str,
    input: Value,
    target: &str,
    warning: &'static str,
    parameter: Option<Parameter>,
) -> Operation {
    Operation {
        label,
        service,
        name,
        input,
        region: None,
        target: target.into(),
        warning,
        parameter,
        paginated: false,
    }
}

pub(super) fn string<'a>(value: &'a Value, fields: &[&str]) -> Option<&'a str> {
    fields
        .iter()
        .find_map(|field| value.get(field).and_then(Value::as_str))
        .filter(|value| !value.is_empty())
}

pub(super) fn nested_string<'a>(value: &'a Value, fields: &[&str]) -> Option<&'a str> {
    let (parent, _) = fields.split_first()?;
    value
        .get(*parent)
        .and_then(|value| string(value, &fields[1..]))
}
