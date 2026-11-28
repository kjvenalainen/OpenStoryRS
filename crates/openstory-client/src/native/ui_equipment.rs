use super::*;
use openstory_protocol::{
    AutoPotionSlot, CharacterLook, InventoryItem, InventorySnapshot, InventoryType,
};
use ui_foundation::{UiRect, UiTextRole, draw_text_role, measure_text};

const WIDTH: usize = EQUIPMENT_INVENTORY_PANEL_WIDTH as usize;
const HEIGHT: usize = EQUIPMENT_INVENTORY_PANEL_HEIGHT as usize;
pub(super) const CLOSE: UiRect = ui_dialog::inventory_close(WIDTH as f32);
pub(super) const ITEM_IGNORE_PENDANT_ID: i32 = 1_812_007;
#[derive(Clone, Copy)]
struct PetControlSlots {
    pet: Option<UiRect>,
    hp: UiRect,
    mp: UiRect,
}

impl PetControlSlots {
    fn potions(self) -> [(AutoPotionSlot, UiRect); 2] {
        [(AutoPotionSlot::Hp, self.hp), (AutoPotionSlot::Mp, self.mp)]
    }
}

#[derive(Clone, Copy, Default)]
pub(super) struct PetSlots {
    pub(super) summoned: Option<(i32, i32)>,
    pub(super) hp: i32,
    pub(super) mp: i32,
}

impl PetSlots {
    fn pet_in_inventory(self, inventory: &InventorySnapshot) -> Option<&InventoryItem> {
        let (item_id, unique_id) = self.summoned?;
        let pets = inventory.items.iter().filter(|item| {
            item.inventory_type == InventoryType::Cash
                && item.item_id == item_id
                && item.pet.is_some()
        });
        pets.clone()
            .find(|item| item.cash_id == Some(i64::from(unique_id)))
            .or_else(|| pets.into_iter().next())
    }

    fn potion(self, slot: AutoPotionSlot) -> i32 {
        match slot {
            AutoPotionSlot::Hp => self.hp,
            AutoPotionSlot::Mp => self.mp,
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub(super) enum EquipmentHover {
    Worn(u16),
    Pet,
    Potion(AutoPotionSlot),
}

impl EquipmentHover {
    pub(super) fn item(
        self,
        inventory: &InventorySnapshot,
        pet: PetSlots,
    ) -> Option<InventoryItem> {
        match self {
            Self::Worn(slot) => inventory.equipped().find(|item| item.slot == slot).cloned(),
            Self::Pet => pet.pet_in_inventory(inventory).cloned().or_else(|| {
                Some(InventoryItem {
                    inventory_type: InventoryType::Cash,
                    item_id: pet.summoned?.0,
                    count: 1,
                    ..Default::default()
                })
            }),
            Self::Potion(slot) => {
                let item_id = pet.potion(slot);
                (item_id > 0).then(|| {
                    inventory
                        .items
                        .iter()
                        .find(|item| {
                            item.inventory_type == InventoryType::Use && item.item_id == item_id
                        })
                        .cloned()
                        .unwrap_or(InventoryItem {
                            inventory_type: InventoryType::Use,
                            item_id,
                            ..Default::default()
                        })
                })
            }
        }
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(super) enum EquipmentTab {
    #[default]
    Equip,
    Deco,
    Beauty,
    Pet,
}

impl EquipmentTab {
    pub(super) const ALL: [Self; 4] = [Self::Equip, Self::Deco, Self::Beauty, Self::Pet];

    pub(super) fn bounds(self) -> UiRect {
        UiRect::new(9.0 + 51.0 * self as usize as f32, 25.0, 50.0, 21.0)
    }

    fn label(self) -> &'static str {
        match self {
            Self::Equip => "Equip",
            Self::Deco => "Deco",
            Self::Beauty => "Beauty",
            Self::Pet => "Pet",
        }
    }

    fn cells(self) -> impl Iterator<Item = GearCell> {
        let cells: &[_] = match self {
            Self::Equip | Self::Deco => &CHARACTER_CELLS,
            Self::Pet => &PET_CELLS,
            Self::Beauty => &[],
        };
        cells
            .iter()
            .copied()
            .filter(move |cell| self != Self::Deco || cell.slot <= 50)
    }

    fn item_in_cell(self, inventory: &InventorySnapshot, cell: GearCell) -> Option<&InventoryItem> {
        if self == Self::Pet {
            return inventory
                .equipped()
                .filter(|item| pet_gear_cell(item.item_id) == Some(cell.slot))
                .min_by_key(|item| item.slot);
        }
        let slot = cell.slot + if self == Self::Deco { 100 } else { 0 };
        inventory
            .equipped()
            .find(|item| !is_pet_gear(item.item_id) && item.slot == slot)
    }

    fn slot_is_blocked(self, inventory: &InventorySnapshot, slot: u16) -> bool {
        let offset = match self {
            Self::Equip => 0,
            Self::Deco => 100,
            Self::Pet | Self::Beauty => return false,
        };
        match slot {
            6 => inventory
                .equipped()
                .any(|item| item.slot == 5 + offset && item.item_id / 10_000 == 105),
            10 if self == Self::Equip => inventory
                .equipped()
                .any(|item| item.slot == 11 && is_two_handed_weapon(item.item_id)),
            _ => false,
        }
    }

    fn pet_controls(self) -> Option<PetControlSlots> {
        match self {
            Self::Equip => Some(PetControlSlots {
                pet: Some(character_cell_bounds(3, 6)),
                hp: character_cell_bounds(4, 6),
                mp: character_cell_bounds(4, 5),
            }),
            Self::Pet => Some(PetControlSlots {
                pet: None,
                hp: pet_cell_bounds(3, 3),
                mp: pet_cell_bounds(3, 2),
            }),
            Self::Deco | Self::Beauty => None,
        }
    }

    pub(super) fn potion_at(self, x: f32, y: f32) -> Option<AutoPotionSlot> {
        self.pet_controls()?
            .potions()
            .into_iter()
            .find_map(|(slot, bounds)| bounds.contains(x, y).then_some(slot))
    }

    pub(super) fn hover(
        self,
        inventory: &InventorySnapshot,
        pet: PetSlots,
        x: f32,
        y: f32,
    ) -> Option<EquipmentHover> {
        if pet.summoned.is_some()
            && self
                .pet_controls()
                .is_some_and(|slots| slots.pet.is_some_and(|bounds| bounds.contains(x, y)))
        {
            return Some(EquipmentHover::Pet);
        }
        if let Some(slot) = self.potion_at(x, y).filter(|slot| pet.potion(*slot) > 0) {
            return Some(EquipmentHover::Potion(slot));
        }
        self.worn_at(inventory, x, y)
            .map(|item| EquipmentHover::Worn(item.slot))
    }

    pub(super) fn worn_at(
        self,
        inventory: &InventorySnapshot,
        x: f32,
        y: f32,
    ) -> Option<&InventoryItem> {
        let cell = self.cells().find(|cell| cell.bounds.contains(x, y))?;
        self.item_in_cell(inventory, cell)
    }

    pub(super) fn hitboxes(self, inventory: &InventorySnapshot) -> Vec<UiRect> {
        let mut bounds = Self::ALL.map(Self::bounds).to_vec();
        bounds.push(CLOSE);
        bounds.extend(
            self.cells()
                .filter(|cell| self.item_in_cell(inventory, *cell).is_some())
                .map(|cell| cell.bounds),
        );
        if let Some(slots) = self.pet_controls() {
            bounds.extend(slots.pet);
            bounds.extend([slots.hp, slots.mp]);
        }
        bounds
    }
}

#[derive(Clone, Copy)]
struct GearCell {
    slot: u16,
    label: &'static str,
    bounds: UiRect,
}

const fn character_cell_bounds(column: u8, row: u8) -> UiRect {
    UiRect::new(
        24.0 + column as f32 * INVENTORY_SLOT_STEP_X as f32,
        56.0 + row as f32 * INVENTORY_SLOT_STEP_Y as f32,
        INVENTORY_SLOT_CELL_WIDTH as f32,
        INVENTORY_SLOT_CELL_HEIGHT as f32,
    )
}

const fn character_cell(slot: u16, label: &'static str, column: u8, row: u8) -> GearCell {
    GearCell {
        slot,
        label,
        bounds: character_cell_bounds(column, row),
    }
}

const CHARACTER_CELLS: [GearCell; 25] = [
    character_cell(1, "Hat", 1, 0),
    character_cell(52, "Book", 4, 0),
    character_cell(49, "Medal", 0, 1),
    character_cell(2, "Face", 1, 1),
    character_cell(57, "Badge", 2, 1),
    character_cell(12, "Ring", 3, 1),
    character_cell(13, "Ring", 4, 1),
    character_cell(3, "Eye", 2, 2),
    character_cell(4, "Earrings", 3, 2),
    character_cell(54, "Shoulder", 4, 2),
    character_cell(9, "Cape", 0, 3),
    character_cell(5, "Top", 1, 3),
    character_cell(17, "Pendant", 2, 3),
    character_cell(11, "Weapon", 3, 3),
    character_cell(10, "Shield", 4, 3),
    character_cell(8, "Gloves", 0, 4),
    character_cell(6, "Bottom", 1, 4),
    character_cell(50, "Belt", 2, 4),
    character_cell(15, "Ring", 3, 4),
    character_cell(16, "Ring", 4, 4),
    character_cell(59, "Heart", 0, 5),
    character_cell(7, "Shoes", 2, 5),
    character_cell(18, "Mount", 0, 6),
    character_cell(19, "Saddle", 1, 6),
    character_cell(55, "Android", 2, 6),
];

const fn pet_cell_bounds(column: u8, row: u8) -> UiRect {
    UiRect::new(
        41.0 + column as f32 * INVENTORY_SLOT_STEP_X as f32,
        53.0 + row as f32 * INVENTORY_SLOT_STEP_Y as f32,
        INVENTORY_SLOT_CELL_WIDTH as f32,
        INVENTORY_SLOT_CELL_HEIGHT as f32,
    )
}

const fn pet_cell(slot: u16, label: &'static str, column: u8, row: u8) -> GearCell {
    GearCell {
        slot,
        label,
        bounds: pet_cell_bounds(column, row),
    }
}

const PET_CELLS: [GearCell; 11] = [
    pet_cell(1812, "HP Pouch", 0, 0),
    pet_cell(1813, "MP Pouch", 1, 0),
    pet_cell(1815, "Binocular", 2, 0),
    pet_cell(1817, "Item Ignore", 3, 0),
    pet_cell(1811, "Item Pouch", 0, 1),
    pet_cell(1810, "Meso Magnet", 1, 1),
    pet_cell(1814, "Wing Boots", 2, 1),
    pet_cell(1800, "Pet Outfit", 3, 1),
    pet_cell(1816, "Magic Scales", 0, 2),
    pet_cell(1820, "Name Tag", 1, 2),
    pet_cell(1830, "Quote Ring", 2, 2),
];

pub(super) fn item_ignore_bounds() -> UiRect {
    pet_cell_bounds(3, 0)
}

fn is_pet_gear(item_id: i32) -> bool {
    (180..=183).contains(&(item_id / 10_000))
}

fn pet_gear_cell(item_id: i32) -> Option<u16> {
    match item_id {
        1_812_000..=1_812_007 => Some(1810 + (item_id - 1_812_000) as u16),
        id if id / 10_000 == 180 => Some(1800),
        id if id / 10_000 == 182 => Some(1820),
        id if id / 10_000 == 183 => Some(1830),
        _ => None,
    }
}

pub(super) fn render(
    font: &fontdue::Font,
    inventory: &InventorySnapshot,
    look: Option<&CharacterLook>,
    tab: EquipmentTab,
    active_scroll_item_id: Option<i32>,
    dragged_equipped_slot: Option<u16>,
    pet: PetSlots,
    ui: &NxArchive,
    items: &NxArchive,
    characters: &NxArchive,
    strings: &NxArchive,
    icon_cache: &mut HashMap<i32, Option<(u32, u32, Arc<[u8]>)>>,
    name_cache: &mut HashMap<i32, Option<String>>,
) -> Vec<u8> {
    let mut pixels = vec![0; WIDTH * HEIGHT * 4];
    let mut dialog = NxDialog::new(&mut pixels, WIDTH, HEIGHT, ui);
    let _ = dialog.inventory_window(
        UiRect::new(0.0, 0.0, WIDTH as f32, HEIGHT as f32),
        font,
        "Equipment Inventory",
        CLOSE,
    );
    for candidate in EquipmentTab::ALL {
        let _ = dialog.labeled_tab(
            candidate.bounds(),
            candidate == tab,
            font,
            candidate.label(),
        );
    }
    let mut panel = EquipmentContent {
        pixels: &mut pixels,
        font,
    };
    panel.fill(UiRect::new(8.0, 46.0, 205.0, 2.0), [108, 109, 175, 255]);
    match tab {
        EquipmentTab::Equip | EquipmentTab::Deco => {
            panel.background_grid(character_cell_bounds(0, 0), 5, 7);
            panel.silhouette(
                &avatar_parts(characters, &CharacterLook::default()),
                UiRect::new(41.0, 67.0, 137.0, 213.0),
            );
            if tab == EquipmentTab::Equip {
                panel.silhouette(
                    &pet_parts(items, pet.summoned.map_or(5_000_000, |(id, _)| id)),
                    UiRect::new(116.0, 213.0, 92.0, 80.0),
                );
            }
        }
        EquipmentTab::Beauty => panel.beauty(look, characters, strings),
        EquipmentTab::Pet => {
            panel.background_grid(pet_cell_bounds(0, 0), 4, 4);
            panel.silhouette(
                &pet_parts(items, pet.summoned.map_or(5_000_000, |(id, _)| id)),
                UiRect::new(49.0, 69.0, 132.0, 151.0),
            );
            panel.pet_status(inventory, pet, ui, items, strings, name_cache);
        }
    }
    for cell in tab.cells() {
        let item = tab
            .item_in_cell(inventory, cell)
            .filter(|item| Some(item.slot) != dragged_equipped_slot);
        let tone = match item {
            Some(item)
                if active_scroll_item_id
                    .and_then(|scroll_id| invalid_scroll_target(scroll_id, item))
                    == Some(true) =>
            {
                SlotTone::InvalidScroll
            }
            Some(item) if item.cash => SlotTone::Cash,
            None if tab.slot_is_blocked(inventory, cell.slot) => SlotTone::Blocked,
            _ => SlotTone::Normal,
        };
        panel.gear(
            cell.label,
            cell.bounds,
            item.map(|item| item.item_id),
            tone,
            items,
            characters,
            icon_cache,
        );
    }
    if let Some(slots) = tab.pet_controls() {
        if let Some(bounds) = slots.pet {
            panel.gear(
                "Pet",
                bounds,
                pet.summoned.map(|(id, _)| id),
                SlotTone::Normal,
                items,
                characters,
                icon_cache,
            );
        }
        for (slot, bounds) in slots.potions() {
            let label = match slot {
                AutoPotionSlot::Hp => "Auto HP",
                AutoPotionSlot::Mp => "Auto MP",
            };
            let id = pet.potion(slot);
            panel.gear(
                label,
                bounds,
                (id > 0).then_some(id),
                SlotTone::Normal,
                items,
                characters,
                icon_cache,
            );
            if id > 0 {
                let count = inventory
                    .items
                    .iter()
                    .filter(|item| item.inventory_type == InventoryType::Use && item.item_id == id)
                    .map(|item| u32::from(item.count))
                    .sum::<u32>();
                let _ = draw_item_stack_count(
                    panel.pixels,
                    WIDTH,
                    HEIGHT,
                    ui,
                    count,
                    bounds.x as i32 + 2,
                    (bounds.y + bounds.height) as i32 - 12,
                    bounds.width as usize - 4,
                );
            }
        }
    }
    pixels
}

#[derive(Clone, Copy)]
enum SlotTone {
    Normal,
    Cash,
    Blocked,
    InvalidScroll,
}

fn invalid_scroll_target(scroll_id: i32, equipment: &InventoryItem) -> Option<bool> {
    if scroll_id / 10_000 != 204 {
        return None;
    }

    let item_category = equipment.item_id / 10_000;
    let target = match scroll_id {
        2_049_000..=2_049_099 => return None,
        2_049_100 => ScrollTarget::AnyEquipment,
        2_049_110..=2_049_119 => ScrollTarget::Category(108),
        2_049_200..=2_049_299 => ScrollTarget::Accessory,
        _ => match (scroll_id % 10_000) / 100 {
            group @ 0..=49 => ScrollTarget::Category(100 + group),
            80 => ScrollTarget::PetEquipment,
            _ => return None,
        },
    };

    if equipment.cash && !matches!(target, ScrollTarget::PetEquipment) {
        return Some(true);
    }
    let Some(equip) = equipment.equip.as_ref() else {
        return Some(true);
    };
    if equip.upgrade_slots == 0 {
        return Some(true);
    }

    Some(match target {
        ScrollTarget::Category(category) => item_category != category,
        ScrollTarget::Accessory => !matches!(item_category, 111..=113),
        ScrollTarget::PetEquipment => !matches!(item_category, 180..=183),
        ScrollTarget::AnyEquipment => false,
    })
}

#[derive(Clone, Copy)]
enum ScrollTarget {
    Category(i32),
    Accessory,
    PetEquipment,
    AnyEquipment,
}

struct EquipmentContent<'a> {
    pixels: &'a mut [u8],
    font: &'a fontdue::Font,
}

impl EquipmentContent<'_> {
    fn fill(&mut self, bounds: UiRect, bgra: [u8; 4]) {
        fill_dialogue_rect(
            self.pixels,
            WIDTH,
            HEIGHT,
            bounds.x as usize,
            bounds.y as usize,
            bounds.width as usize,
            bounds.height as usize,
            bgra,
        );
    }

    fn text(&mut self, text: &str, bounds: UiRect, role: UiTextRole, rgb: [u8; 3]) {
        draw_text_role(
            self.font,
            self.pixels,
            WIDTH,
            HEIGHT,
            text,
            bounds.x as i32,
            bounds.y as i32,
            role,
            bounds.width as usize,
            bounds.height as usize,
            Some(rgb),
        );
    }

    fn background_grid(&mut self, cell: UiRect, columns: u8, rows: u8) {
        let step_x = ui_foundation::INVENTORY_GRID.step_x;
        let step_y = ui_foundation::INVENTORY_GRID.step_y;
        let grid = UiRect::new(
            cell.x - 1.0,
            cell.y - 1.0,
            step_x * f32::from(columns) + 1.0,
            step_y * f32::from(rows) + 1.0,
        );
        self.fill(grid, [250, 248, 245, 255]);
        for column in 0..=columns {
            self.fill(
                UiRect::new(
                    grid.x + f32::from(column) * step_x,
                    grid.y,
                    1.0,
                    grid.height,
                ),
                [236, 230, 222, 255],
            );
        }
        for row in 0..=rows {
            self.fill(
                UiRect::new(grid.x, grid.y + f32::from(row) * step_y, grid.width, 1.0),
                [236, 230, 222, 255],
            );
        }
    }

    fn overlay(&mut self, bounds: UiRect, bgra: [u8; 4]) {
        let width = bounds.width as usize;
        let height = bounds.height as usize;
        composite_bgra_pixels(
            self.pixels,
            WIDTH,
            HEIGHT,
            &bgra.repeat(width * height),
            width,
            height,
            bounds.x as i32,
            bounds.y as i32,
        );
    }

    fn gear(
        &mut self,
        label: &str,
        bounds: UiRect,
        id: Option<i32>,
        tone: SlotTone,
        items: &NxArchive,
        characters: &NxArchive,
        icon_cache: &mut HashMap<i32, Option<(u32, u32, Arc<[u8]>)>>,
    ) {
        let (edge, fill) = match (tone, id.is_some()) {
            (SlotTone::Blocked, _) => ([149, 139, 177, 186], [171, 161, 203, 131]),
            (SlotTone::InvalidScroll, _) => ([44, 44, 232, 255], [68, 68, 204, 110]),
            (SlotTone::Cash, _) => ([182, 157, 140, 187], [211, 187, 168, 153]),
            (_, true) => ([174, 151, 132, 191], [208, 187, 166, 147]),
            (_, false) => ([187, 170, 152, 153], [221, 203, 187, 120]),
        };
        self.overlay(bounds, fill);
        let right = bounds.x + bounds.width - 1.0;
        let bottom = bounds.y + bounds.height - 1.0;
        for border in [
            UiRect::new(bounds.x, bounds.y, bounds.width, 1.0),
            UiRect::new(bounds.x, bottom, bounds.width, 1.0),
            UiRect::new(bounds.x, bounds.y + 1.0, 1.0, bounds.height - 2.0),
            UiRect::new(right, bounds.y + 1.0, 1.0, bounds.height - 2.0),
        ] {
            self.overlay(border, edge);
        }
        self.overlay(
            UiRect::new(bounds.x + 1.0, bounds.y + 1.0, bounds.width - 2.0, 1.0),
            [255, 255, 255, 110],
        );

        let base_size = 8.0;
        let label = label.to_ascii_uppercase();
        let available = bounds.width - 4.0;
        let size =
            base_size * ((available - 2.0) / measure_text(self.font, &label, base_size)).min(1.0);
        draw_dialogue_text(
            self.font,
            self.pixels,
            WIDTH,
            HEIGHT,
            &label,
            bounds.x as i32 + 2,
            bounds.y as i32 + 1,
            size,
            available as usize,
            10,
            if matches!(tone, SlotTone::Blocked) {
                [110, 71, 91]
            } else if matches!(tone, SlotTone::InvalidScroll) {
                [194, 38, 44]
            } else {
                [52, 74, 97]
            },
        );
        if let Some(id) = id {
            self.item_icon(
                id,
                UiRect::new(
                    bounds.x + 2.0,
                    bounds.y + 7.0,
                    bounds.width - 4.0,
                    bounds.height - 9.0,
                ),
                items,
                characters,
                icon_cache,
            );
            if matches!(tone, SlotTone::Cash) {
                self.cash_marker(
                    bounds.x + bounds.width - 8.0,
                    bounds.y + bounds.height - 8.0,
                );
            }
        }
    }

    fn cash_marker(&mut self, x: f32, y: f32) {
        self.fill(UiRect::new(x + 1.0, y, 5.0, 7.0), [52, 124, 183, 255]);
        self.fill(UiRect::new(x, y + 1.0, 7.0, 5.0), [52, 124, 183, 255]);
        self.fill(UiRect::new(x + 1.0, y + 1.0, 5.0, 5.0), [63, 195, 251, 255]);
        self.fill(
            UiRect::new(x + 2.0, y + 2.0, 2.0, 2.0),
            [168, 238, 255, 255],
        );
    }

    fn silhouette(&mut self, parts: &[DecodedSprite], bounds: UiRect) {
        if parts.is_empty() {
            return;
        }
        let mut mask = vec![0; WIDTH * HEIGHT * 4];
        EquipmentContent {
            pixels: &mut mask,
            font: self.font,
        }
        .avatar(parts, bounds);
        let alpha = |x: usize, y: usize| mask[(y * WIDTH + x) * 4 + 3];
        let left = (bounds.x as usize).max(2) - 1;
        let top = (bounds.y as usize).max(2) - 1;
        let right = ((bounds.x + bounds.width) as usize + 1).min(WIDTH - 1);
        let bottom = ((bounds.y + bounds.height) as usize + 1).min(HEIGHT - 1);
        let mut watermarked = Vec::with_capacity((right - left) * (bottom - top) * 4);
        for y in top..bottom {
            for x in left..right {
                let center = alpha(x, y);
                let neighbor = alpha(x + 1, y)
                    .max(alpha(x - 1, y))
                    .max(alpha(x, y + 1))
                    .max(alpha(x, y - 1));
                let opacity = (center / 6).saturating_add(neighbor.saturating_sub(center) / 2);
                watermarked.extend_from_slice(&[178, 150, 123, opacity]);
            }
        }
        composite_bgra_pixels(
            self.pixels,
            WIDTH,
            HEIGHT,
            &watermarked,
            right - left,
            bottom - top,
            left as i32,
            top as i32,
        );
    }

    fn item_icon(
        &mut self,
        item_id: i32,
        bounds: UiRect,
        items: &NxArchive,
        characters: &NxArchive,
        cache: &mut HashMap<i32, Option<(u32, u32, Arc<[u8]>)>>,
    ) {
        if let Some((width, height, icon)) = inventory_item_icon(item_id, items, characters, cache)
        {
            let scale = (bounds.width / width as f32)
                .min(bounds.height / height as f32)
                .min(1.0);
            let w = (width as f32 * scale).round().max(1.0);
            let h = (height as f32 * scale).round().max(1.0);
            composite_scaled_nx_icon(
                self.pixels,
                WIDTH,
                HEIGHT,
                &icon,
                width as usize,
                height as usize,
                (bounds.x + (bounds.width - w) / 2.0).round() as i32,
                (bounds.y + (bounds.height - h) / 2.0).round() as i32,
                w as usize,
                h as usize,
            );
        }
    }

    fn avatar(&mut self, parts: &[DecodedSprite], bounds: UiRect) {
        let Some(first) = parts.first() else {
            return;
        };
        let (left, top, right, bottom) = parts.iter().fold(
            (first.rect.x, first.rect.y, first.rect.x, first.rect.y),
            |(left, top, right, bottom), part| {
                (
                    left.min(part.rect.x),
                    top.min(part.rect.y),
                    right.max(part.rect.x + part.rect.width),
                    bottom.max(part.rect.y + part.rect.height),
                )
            },
        );
        let scale = (bounds.width / (right - left)).min(bounds.height / (bottom - top));
        let x = bounds.x + (bounds.width - (right - left) * scale) / 2.0;
        let y = bounds.y + (bounds.height - (bottom - top) * scale) / 2.0;
        for part in parts {
            composite_scaled_nx_icon(
                self.pixels,
                WIDTH,
                HEIGHT,
                &part.pixels,
                part.texture_width as usize,
                part.texture_height as usize,
                (x + (part.rect.x - left) * scale).round() as i32,
                (y + (part.rect.y - top) * scale).round() as i32,
                (part.rect.width * scale).round() as usize,
                (part.rect.height * scale).round() as usize,
            );
        }
    }

    fn beauty(
        &mut self,
        look: Option<&CharacterLook>,
        characters: &NxArchive,
        strings: &NxArchive,
    ) {
        let Some(look) = look else {
            self.text(
                "Appearance unavailable.",
                UiRect::new(16.0, 62.0, 190.0, 16.0),
                UiTextRole::Value,
                [54, 71, 87],
            );
            return;
        };
        for (row, label, category, id) in [
            (0, "Hair", "Hair", look.hair_id),
            (1, "Eyes", "Face", look.face_id),
            (2, "Skin", "Skin", i32::from(look.skin)),
        ] {
            let y = 60.0 + row as f32 * 68.0;
            self.fill(UiRect::new(13.0, y, 195.0, 61.0), [214, 202, 187, 255]);
            self.fill(
                UiRect::new(14.0, y + 1.0, 193.0, 59.0),
                [245, 240, 235, 255],
            );
            self.fill(UiRect::new(18.0, y + 5.0, 69.0, 51.0), [233, 221, 207, 255]);
            let mut parts = avatar_parts(
                characters,
                &CharacterLook {
                    skin: look.skin,
                    hair_id: if row == 0 { look.hair_id } else { 0 },
                    face_id: if row == 1 { look.face_id } else { 0 },
                    ..Default::default()
                },
            );
            // The base avatar's head, hair and face layers are independent of clothing.
            parts.retain(|part| matches!(part.depth_order, 0 | 3 | 16..=19 | 23 | 24));
            self.avatar(&parts, UiRect::new(27.0, y + 7.0, 51.0, 47.0));
            self.text(
                label,
                UiRect::new(98.0, y + 9.0, 102.0, 17.0),
                UiTextRole::Label,
                [38, 53, 69],
            );
            let name = nx_string_value(strings, &format!("Eqp.img/Eqp/{category}/{id}/name"))
                .unwrap_or_else(|| {
                    if category == "Skin" {
                        format!("Tone {id}")
                    } else {
                        format!("Style {id}")
                    }
                });
            self.text(
                &name,
                UiRect::new(98.0, y + 31.0, 102.0, 15.0),
                UiTextRole::Value,
                [60, 76, 93],
            );
        }
        self.text(
            "Visit a salon to change your look.",
            UiRect::new(17.0, 272.0, 187.0, 16.0),
            UiTextRole::Value,
            [54, 71, 87],
        );
    }

    fn pet_status(
        &mut self,
        inventory: &InventorySnapshot,
        slots: PetSlots,
        ui: &NxArchive,
        items: &NxArchive,
        strings: &NxArchive,
        name_cache: &mut HashMap<i32, Option<String>>,
    ) {
        const STATUS_Y: f32 = 203.0;
        let portrait = UiRect::new(12.0, STATUS_Y, 74.0, 98.0);
        const LABEL_X: f32 = 89.0;
        const VALUE_X: f32 = 140.0;
        const ROW_Y: f32 = STATUS_Y;
        const ROW_HEIGHT: f32 = 16.0;
        let native = NxDialog::new(self.pixels, WIDTH, HEIGHT, ui)
            .frame_bitmap("UIWindow.img/UserInfo/backgrnd2");

        let labels = ["Type", "Level", "Closeness", "Fullness"];
        let label_sources = [(5, 329), (5, 347), (137, 329), (137, 347)];
        let pet = slots
            .pet_in_inventory(inventory)
            .and_then(|item| item.pet.as_ref());
        let species = slots
            .summoned
            .map(|(id, _)| item_name(id, strings, name_cache));
        let name = pet
            .map(|pet| pet.name.as_str())
            .filter(|name| !name.trim().is_empty())
            .or(species.as_deref())
            .unwrap_or("No pet");
        let values = [
            species.as_deref().unwrap_or("—").to_owned(),
            pet.map_or_else(|| "—".to_owned(), |pet| pet.level.to_string()),
            pet.map_or_else(|| "—".to_owned(), |pet| pet.closeness.to_string()),
            pet.map_or_else(|| "—".to_owned(), |pet| pet.fullness.to_string()),
        ];

        if let Some((width, _, source)) = native.as_ref().filter(|(w, h, _)| (*w, *h) == (275, 399))
        {
            // Reassemble the native Character Info pet artwork in a compact two-column card.
            self.status_art(source, *width, (5, 214, 96, 112), portrait);
            for (index, (x, y)) in label_sources.into_iter().enumerate() {
                let row_y = ROW_Y + index as f32 * ROW_HEIGHT;
                self.status_art(
                    source,
                    *width,
                    (x, y, 49, 16),
                    UiRect::new(LABEL_X, row_y, 49.0, 15.0),
                );
                self.status_art(
                    source,
                    *width,
                    (55, 329, 80, 16),
                    UiRect::new(VALUE_X, row_y, 69.0, 15.0),
                );
            }
            self.status_art(
                source,
                *width,
                (104, 214, 147, 37),
                UiRect::new(LABEL_X, ROW_Y + 4.0 * ROW_HEIGHT + 2.0, 120.0, 32.0),
            );
        } else {
            self.fill(portrait, [216, 221, 223, 255]);
            self.fill(
                UiRect::new(14.0, STATUS_Y + 79.0, 70.0, 16.0),
                [104, 96, 87, 255],
            );
            for (index, label) in labels.into_iter().enumerate() {
                let row_y = ROW_Y + index as f32 * ROW_HEIGHT;
                self.fill(UiRect::new(LABEL_X, row_y, 49.0, 15.0), [124, 79, 174, 255]);
                self.text(
                    label,
                    UiRect::new(LABEL_X + 2.0, row_y + 1.0, 47.0, 12.0),
                    UiTextRole::Small,
                    [255, 255, 255],
                );
                self.fill(
                    UiRect::new(VALUE_X, row_y, 69.0, 15.0),
                    [214, 220, 224, 255],
                );
            }
        }
        if let Some((id, _)) = slots.summoned {
            self.avatar(
                &pet_parts(items, id),
                UiRect::new(25.0, STATUS_Y + 14.0, 48.0, 62.0),
            );
        }
        let name = ui_foundation::ellipsize(self.font, name, 10.0, 64.0);
        let name_left =
            (portrait.x + (portrait.width - measure_text(self.font, &name, 10.0)) / 2.0).round();
        draw_dialogue_text(
            self.font,
            self.pixels,
            WIDTH,
            HEIGHT,
            &name,
            name_left as i32,
            STATUS_Y as i32 + 80,
            10.0,
            68,
            13,
            [255, 255, 255],
        );
        for (index, value) in values.iter().enumerate() {
            self.text(
                value,
                UiRect::new(
                    VALUE_X + 4.0,
                    ROW_Y + index as f32 * ROW_HEIGHT + 1.0,
                    63.0,
                    14.0,
                ),
                UiTextRole::Value,
                [35, 35, 35],
            );
        }
    }

    fn status_art(
        &mut self,
        source: &[u8],
        source_width: usize,
        (x, y, width, height): (usize, usize, usize, usize),
        bounds: UiRect,
    ) {
        let pixels: Vec<_> = source
            .chunks_exact(source_width * 4)
            .skip(y)
            .take(height)
            .flat_map(|row| row[x * 4..(x + width) * 4].iter().copied())
            .collect();
        composite_scaled_nx_icon(
            self.pixels,
            WIDTH,
            HEIGHT,
            &pixels,
            width,
            height,
            bounds.x as i32,
            bounds.y as i32,
            bounds.width as usize,
            bounds.height as usize,
        );
    }
}

fn avatar_parts(archive: &NxArchive, look: &CharacterLook) -> Vec<DecodedSprite> {
    let mut parts = Vec::new();
    let _ = append_character_avatar(
        &mut parts,
        archive,
        look,
        "stand1",
        0,
        0.0,
        0.0,
        &mut MapArtCache::default(),
    );
    parts
}

fn pet_parts(items: &NxArchive, item_id: i32) -> Vec<DecodedSprite> {
    let mut parts = Vec::new();
    let mut cache = MapArtCache::default();
    for stance in ["stand0", "stand1"] {
        let path = format!("Pet/{item_id}.img/{stance}/0");
        if let Some(node) = items.root().resolve(&path).ok().flatten() {
            let _ =
                append_map_art_sprite(&mut parts, &node, &path, 0.0, 0.0, 255, false, &mut cache);
            if !parts.is_empty() {
                break;
            }
        }
    }
    parts
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn worn_layers_hit_the_same_hat_cell_and_pet_gear_stays_in_pet() {
        let inventory = InventorySnapshot {
            items: [(1, 1_002_000), (101, 1_002_187), (115, 1_812_000)]
                .map(|(slot, item_id)| InventoryItem {
                    inventory_type: InventoryType::Equipped,
                    slot,
                    item_id,
                    ..Default::default()
                })
                .to_vec(),
            ..Default::default()
        };
        let hat = character_cell_bounds(1, 0);
        let pet = PetSlots::default();
        assert!(
            EquipmentTab::Equip.hover(&inventory, pet, hat.x, hat.y)
                == Some(EquipmentHover::Worn(1))
        );
        assert!(
            EquipmentTab::Deco.hover(&inventory, pet, hat.x, hat.y)
                == Some(EquipmentHover::Worn(101))
        );
        let ring = character_cell_bounds(3, 4);
        assert!(
            EquipmentTab::Deco
                .hover(&inventory, pet, ring.x, ring.y)
                .is_none()
        );
        let magnet = pet_cell_bounds(1, 1);
        assert!(
            EquipmentTab::Pet.hover(&inventory, pet, magnet.x, magnet.y)
                == Some(EquipmentHover::Worn(115))
        );
    }

    #[test]
    fn both_pet_views_accept_drops_into_empty_potion_slots() {
        for tab in [EquipmentTab::Equip, EquipmentTab::Pet] {
            let slots = tab.pet_controls().unwrap();
            assert!(slots.mp.y < slots.hp.y);
            assert_eq!(
                tab.potion_at(slots.hp.x, slots.hp.y),
                Some(AutoPotionSlot::Hp)
            );
            assert_eq!(
                tab.potion_at(slots.mp.x, slots.mp.y),
                Some(AutoPotionSlot::Mp)
            );
            assert_eq!(slots.pet.is_some(), tab == EquipmentTab::Equip);
            if let Some(pet_bounds) = slots.pet {
                assert!(pet_bounds.x < slots.hp.x);
                assert_eq!(pet_bounds.y, slots.hp.y);
                assert_eq!(tab.potion_at(pet_bounds.x, pet_bounds.y), None);
            }
            assert!(tab.cells().all(|cell| {
                slots
                    .pet
                    .into_iter()
                    .chain([slots.hp, slots.mp])
                    .all(|target| !cell.bounds.contains(target.x, target.y))
            }));
        }
        assert!(EquipmentTab::Beauty.pet_controls().is_none());
        assert!(EquipmentTab::Deco.pet_controls().is_none());
    }
}
