# UI.nx visual asset index

This index inventories the existing v83 `UI.nx` used by this project. Start with
this guide to find a panel family, then search [UI_ASSETS.tsv](UI_ASSETS.tsv) for
exact paths, bitmap dimensions, and available placement and animation metadata.
The TSV contains **11,080 bitmap nodes** under 16 top-level images, including
variants and individual animation frames. It describes available artwork, not
which features are implemented or which sprites should be composed together.

The source was `../OpenStory/wz/UI.nx`, SHA-256
`0340c611f1f330fad1a3d4bcf05cc89f96c122c113ac584b2c0cd07c3e63d66e`.
Other versions of the archive can differ. Assets remain in the existing data
directory; this index contains no bitmap data.

## Find artwork for a panel

| Task | Start here in `UI.nx` |
| --- | --- |
| Shared buttons, scrollbars, sliders, checkboxes, cursors, numeric glyphs | `Basic.img` |
| Login, world/channel selection, character selection and creation, PIN, terms | `Login.img` |
| Item and equipment inventory, skills, stats, quests, key configuration | `UIWindow.img/Item`, `Equip`, `Skill`, `Stat`, `Quest`, `KeyConfig` |
| Friends, party, guild and alliance; character information | `UIWindow.img/UserList`, `UserInfo`; guild emblem pieces in `GuildMark.img` |
| NPC shop and storage, tooltips, minimap, world map controls | `UIWindow.img/Shop`, `Trunk`, `ToolTip`, `MiniMap`, `WorldMap` |
| Monster Book, games, events, wedding, notices and megaphones | `UIWindow.img/MonsterBook`, `Minigame`, `EventWindow`, `Wedding`, `Notice`, `FloatNotice`, `Megaphone` |
| HUD, quick slots, gauges and chat frame | `StatusBar.img`; speech bubbles in `ChatBalloon.img`, nameplates in `NameTag.img` |
| Cash shop, trade/cash marketplace, guild bulletin board | `CashShop.img`, `ITC.img`, `GuildBBS.img` |
| Tutorial, buffs, TV and startup logos | `tutorial.img`, `BuffIcon.img`, `MapleTV.img`, `Logo.img` |

The directory names below are archive names rather than claims about behavior.
Some panels also need artwork or data from other archives (for example, item
icons, world map imagery, or character sprites), runtime text, or composed
controls. This index only covers bitmap nodes physically present in `UI.nx`.

## Reading and using the index

- Paths are relative to the UI archive root, with exact spelling and case. For
  example, `UIWindow.img/Item/backgrnd` resolves directly through the existing
  NX reader. Search with `rg '^UIWindow\.img/Item/' docs/UI_ASSETS.tsv`.
- Each TSV row is one bitmap node. `width` and `height` are stored pixel
  dimensions. `origin_x,y` is the bitmap's direct `origin` vector, when one
  exists; a blank means no such direct vector was found, not necessarily `(0,0)`.
  `delay_ms` is the direct `delay` integer, when present. Other positioning,
  animation, and composition metadata is not indexed; inspect the NX tree and
  existing renderer for the full context.
- Button states commonly appear under `normal`, `mouseOver`, `pressed`, and
  `disabled`, often with numbered frames. Other naming schemes exist: inspect
  the specific path rather than assuming all controls have every state.
- Search nearby implementation in `crates/openstory-client/src/native/ui_*.rs`
  for existing compositions and input behavior. Shared controls are in
  `ui_foundation.rs` and `ui_controls.rs`. See [UI_PREVIEW.md](UI_PREVIEW.md)
  and `ui_catalog.rs` for preview cases; render UI changes to
  `target/ui-previews` and inspect the resulting image.
- Counts below are bitmap nodes, not unique pixel payloads: different paths can
  reference the same bitmap. Group names are immediate children that contain at
  least one bitmap; nonvisual metadata and branches with no bitmaps are omitted.

## Archive groups

### `Basic.img` — 578 bitmap nodes

`BtCancel` (5), `BtCancel2` (5), `BtCancel3` (5), `BtClaim` (4), `BtClose` (4), `BtClose2` (4), `BtCoin` (7), `BtComboBox` (4), `BtDecide` (5), `BtDel` (5), `BtDown` (4), `BtHide` (4), `BtHide2` (4), `BtHide3` (4), `BtMacro` (5), `BtMax` (4), `BtMax2` (4), `BtMin` (4), `BtMin2` (4), `BtNo` (5), `BtNo2` (5), `BtOK` (5), `BtOK2` (5), `BtOK3` (5), `BtQGiveup` (5), `BtReceive` (5), `BtSend` (5), `BtUP` (4), `BtYes` (5), `BtYes2` (5), `CheckBox` (4), `ComboBox` (15), `ComboBox2` (4), `ComboBox3` (15), `ComboBox4` (15), `ComboBox5` (15), `Cursor` (43), `HScr` (10), `HScr2` (10), `HScr3` (13), `HScr4` (13), `HScr5` (13), `HScr6` (10), `ItemNo` (10), `KeyDownBar` (3), `KeyDownBar1` (3), `LevelNo` (11), `Notice` (1), `Notice2` (3), `Notice3` (3), `Notice4` (4), `Notice5` (4), `PQuestRank` (6), `ShowLevel` (34), `Slider` (7), `Tab` (9), `Tab2` (9), `Tab3` (9), `Tab4` (9), `Tab5` (9), `Tab6` (9), `Tab7` (9), `VScr` (10), `VScr2` (10), `VScr3` (13), `VScr4` (13), `VScr5` (13), `VScr6` (10), `VScr7` (11), `VScr8` (13), `YesNo` (1), `YesNo2` (3), `YesNo3` (3), `dcMark` (4), `icon` (3)

### `BuffIcon.img` — 11 bitmap nodes

`buff` (9), `united` (2)

### `CashShop.img` — 300 bitmap nodes

`Base` (24), `CS3thEvent` (1), `CSChangeName` (6), `CSChar` (12), `CSDiscount` (17), `CSEffect` (29), `CSGift` (17), `CSIcon` (2), `CSInventory` (20), `CSItemSearch` (47), `CSList` (22), `CSLocker` (8), `CSMemberShop` (22), `CSNotice` (13), `CSStatus` (16), `CSTab` (9), `CSTransferWorld` (2), `CSWebMsg` (2), `CashItem` (4), `GuideWords` (4), `NewItem` (4), `PicturePlate` (18), `PrepaidCashItem` (1)

### `ChatBalloon.img` — 863 bitmap nodes

`0` (10), `1` (10), `10` (10), `11` (10), `12` (10), `13` (10), `14` (10), `15` (10), `16` (10), `17` (10), `18` (10), `19` (10), `2` (10), `20` (10), `21` (30), `22` (10), `23` (10), `24` (10), `25` (10), `26` (10), `27` (10), `28` (10), `29` (10), `3` (10), `30` (10), `31` (10), `32` (10), `33` (10), `34` (10), `35` (10), `36` (10), `37` (10), `38` (10), `39` (10), `4` (10), `5` (10), `6` (10), `7` (10), `8` (10), `9` (10), `adboard` (10), `dead` (10), `miniroom` (26), `mob` (19), `npc` (10), `pet` (358), `tutorial` (10)

### `DialogImage.img` — 3 bitmap nodes

`Help` (3)

### `GuildBBS.img` — 59 bitmap nodes

`GuildBBS` (59)

### `GuildMark.img` — 2304 bitmap nodes

`BackGround` (496), `Mark` (1808)

### `ITC.img` — 167 bitmap nodes

`Auction` (6), `Base` (18), `BtAuction` (4), `BtCharge` (4), `BtCheck` (4), `BtExit` (4), `BtSearch` (4), `BtSell` (4), `Buy` (30), `MyPage` (23), `PicturePlate` (3), `Sell` (40), `Sort` (18), `Tab` (5)

### `Login.img` — 1087 bitmap nodes

`BtCancel` (4), `BtOk` (4), `CharSelect` (92), `Common` (182), `GameGrade` (1), `Gender` (22), `NewChar` (54), `NewCharAran` (54), `NewCharKnight` (54), `Notice` (114), `NoticeScroll1` (3), `NoticeScroll2` (3), `Pincode` (28), `RaceSelect` (33), `TOS` (1), `Title` (58), `ViewAllChar` (39), `WorldNotice` (45), `WorldSelect` (296)

### `Logo.img` — 198 bitmap nodes

`Nexon` (137), `Wizet` (61)

### `MapLogin.img` — 1 bitmap nodes

`miniMap` (1)

### `MapleTV.img` — 112 bitmap nodes

`TVbasic` (1), `TVchat` (1), `TVchat1` (7), `TVchat2` (7), `TVmedia` (69), `TVoff` (11), `TVon` (16)

### `NameTag.img` — 420 bitmap nodes

`10` (3), `11` (3), `12` (3), `13` (3), `14` (3), `15` (3), `16` (3), `17` (3), `18` (3), `19` (3), `20` (3), `21` (3), `22` (3), `23` (3), `24` (3), `25` (3), `26` (3), `27` (3), `28` (3), `3` (3), `4` (3), `5` (3), `6` (3), `7` (3), `8` (3), `9` (3), `medal` (240), `pet` (102)

### `StatusBar.img` — 121 bitmap nodes

`BtClaim` (4), `BtMenu` (4), `BtNPT` (4), `BtShop` (4), `BtShort` (4), `BtWhisper` (4), `EquipKey` (6), `FontMemo` (10), `InvenKey` (6), `KeySet` (6), `QuickSlot` (6), `QuickSlotD` (6), `SkillKey` (6), `StatKey` (6), `base` (9), `gauge` (14), `key` (8), `number` (14)

### `UIWindow.img` — 4511 bitmap nodes

`Admin` (1), `AdminClaim` (20), `AranSkillGuide` (8), `AriantMatch` (20), `ArtMegaphone` (9), `AvatarMegaphone` (9), `BetaEdition` (1), `Book` (13), `BtUIClose` (4), `BtUIClose2` (4), `CancelRequests` (31), `CashGachapon` (43), `CashTradingRoom` (22), `Channel` (66), `CharacterClone` (39), `Claim` (20), `ContextMenu` (47), `Coupon` (3), `Delivery` (25), `DualMobGauge` (4), `EnchantSkill` (30), `EnergyBar` (12), `EntrustedShop` (19), `Equip` (41), `EventWindow` (13), `ExceptionItemSearch` (14), `FadeYesNo` (25), `Family` (38), `FamilyTree` (26), `FloatNotice` (132), `GMBoard` (20), `GameMenu` (21), `GameOpt` (1), `IconBase` (1), `Incubator` (9), `InitialQuiz` (22), `Item` (61), `ItemMegaphone` (1), `ItemMove` (16), `ItemProtector` (10), `KarmaScissors` (1), `KeyConfig` (221), `Macro` (28), `MacroSkill` (17), `Maker` (26), `MapleEvent` (10), `MapleTV` (14), `MateMessage` (14), `Megaphone` (21), `MemberShop` (13), `Memo` (33), `Messenger` (17), `MiniMap` (26), `Minigame` (180), `MinigameTable` (50), `MobGage` (206), `MonsterBook` (122), `MonsterCarnival` (20), `MonsterKilling` (98), `MultiLine` (6), `NewYearsCard` (21), `Notice` (9), `PartyRace` (65), `PartySearch` (43), `PersonalShop` (75), `Quest` (155), `QuestAlarm` (16), `QuestIcon` (27), `Radio` (6), `Ranking` (3), `RemoteGachapon` (41), `Report` (1), `Reset` (38), `RpsGame` (41), `Shop` (42), `ShortCut` (45), `Skill` (54), `SkillMacro` (22), `SkillUp` (5), `SlideMenu` (112), `SoftKeyboard` (198), `SpeedQuiz` (27), `Stat` (102), `StoreBank` (10), `SysOpt` (1), `Teleport` (14), `TemporaryStatView` (1), `Title` (27), `ToolTip` (135), `TradingRoom` (21), `Transform` (9), `TreasureBox` (6), `Trunk` (45), `UserInfo` (103), `UserList` (444), `UtilDlgEx` (60), `UtilDlgEx_Avatar` (23), `UtilDlgEx_MultiPetEquip` (1), `UtilDlgEx_Pet` (1), `VegaSpell` (77), `ViciousHammer` (28), `Wedding` (76), `WorldMap` (9), `classMatch` (10), `counsel` (31), `createCygnus` (17), `itemSearch` (51), `muruengRaid` (23), `plant` (10), `raise` (106)

### `tutorial.img` — 345 bitmap nodes

`0` (1), `1` (1), `10` (2), `11` (17), `12` (1), `13` (1), `14` (12), `15` (18), `16` (13), `17` (18), `18` (6), `19` (2), `2` (1), `20` (15), `21` (33), `22` (10), `23` (18), `24` (1), `25` (1), `26` (34), `27` (6), `28` (24), `29` (25), `3` (12), `31` (1), `32` (1), `4` (5), `5` (11), `6` (13), `7` (18), `8` (18), `9` (6)

