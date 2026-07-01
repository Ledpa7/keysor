fn main() {
    let automation = uiautomation::core::UIAutomation::new().unwrap();
    let root = automation.get_root_element().unwrap();
    let id = root.get_automation_id();
    let name = root.get_name();
    println!("Automation ID: {:?}, Name: {:?}", id, name);
}

