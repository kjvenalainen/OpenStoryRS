use super::*;

#[test]
#[ignore = "requires OPENSTORY_DATA_DIR; writes target/ui-previews menu comparison screenshot"]
fn render_menu_comparison_preview() {
    let data_dir = std::env::var("OPENSTORY_DATA_DIR").expect("set OPENSTORY_DATA_DIR");
    let archive = |name: &str| NxArchive::open(Path::new(&data_dir).join(name)).unwrap();
    let map = archive("Map.nx");
    let ui = archive("UI.nx");
    let characters = archive("Character.nx");
    let npcs = archive("Npc.nx");
    let skills = archive("Skill.nx");
    let etc = archive("Etc.nx");
    let items = archive("Item.nx");
    let mobs = archive("Mob.nx");
    let quests = archive("Quest.nx");
    let strings = archive("String.nx");
    let font = load_system_font().unwrap();
    let output = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../target/ui-previews");
    fs::create_dir_all(&output).unwrap();

    write_menu_comparison_preview(
        &output,
        &map,
        &ui,
        &ui,
        &characters,
        &npcs,
        &skills,
        &etc,
        &items,
        &mobs,
        &quests,
        &strings,
        &font,
    )
    .unwrap();
}
