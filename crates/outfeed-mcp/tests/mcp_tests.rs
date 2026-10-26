use outfeed_mcp::schema::list_tools;

#[test]
fn test_mcp_list_tools() {
    let tools = list_tools();
    assert!(tools.is_array());
    let arr = tools.as_array().expect("array");
    assert_eq!(arr.len(), 6);
    let names: Vec<_> = arr
        .iter()
        .filter_map(|t| t.get("name").and_then(|n| n.as_str()))
        .collect();
    assert!(names.contains(&"outfeed_plan"));
    assert!(names.contains(&"outfeed_apply"));
    assert!(names.contains(&"outfeed_install"));
    assert!(names.contains(&"outfeed_push"));
    assert!(names.contains(&"outfeed_publish"));
    assert!(names.contains(&"outfeed_clean"));
}
