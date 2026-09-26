import QtQuick
import QtTest
import "../../qml" as Lunchbox

TestCase {
    id: testCase
    name: "GamePlayHero"
    when: windowShown
    visible: true
    property bool requestedTranslationEnabled: false

    Component {
        id: heroComponent
        Lunchbox.GamePlayHero {
            width: 440
            gameId: "game-1"
            local: true
            loading: false
            canLaunch: true
            discoveryBusy: false
            launchBusy: false
            gameRunning: false
            sessionStopping: false
            sessionTitle: ""
            preparable: false
            prepareBusy: false
            emulatorName: "RetroArch · Mesen"
            platform: "Nintendo Entertainment System"
            launchStatus: "Ready"
            preferenceScope: ""
            emulatorOptionCount: 2
            selectedEmulatorOption: 0
            translationOptedIn: false
            translationFeatureEnabled: true
            firmwareMissingCount: 0
            firmwareSetupLabel: "Set up emulator"
            emulatorLabelAt: function(index) {
                return index === 0 ? "RetroArch · Mesen" : "Mesen"
            }
            emulatorOptionKindAt: function(index) {
                return index === 0 ? "retroarch" : "standalone"
            }
            emulatorOptionStarredAt: function(index) {
                return index === 1
            }
            displayScope: "game"
            displayFullscreen: ""
            displayShader: ""
            displayBezel: ""
            displaySaveStates: ""
            displayInheritedFullscreenLabel: "Inherit → Off (RetroArch default)"
            displayInheritedShaderLabel: "Inherit → Off (RetroArch default)"
            displayInheritedBezelLabel: "Inherit → System pack"
            displayInheritedSaveStatesLabel: "Inherit → Save + resume"
            displayEffectiveSummary: ""
            displayRevision: 0
            displayFullscreenSupported: true
            displayShaderSupported: true
            displayBezelSupported: true
            displaySaveStatesSupported: false
            displayShaderPresetCount: function() { return 0 }
            displayShaderPresetIdAt: function(index) { return "" }
            displayShaderPresetLabelAt: function(index) { return "" }
            displayBezelChoiceCount: function() { return 6 }
            displayBezelChoiceIdAt: function(index) {
                return ["system", "themed", "orionsangel", "orionsangel-plain",
                        "ultrawide", "ultrawide-night"][index]
            }
            displayBezelChoiceLabelAt: function(index) {
                return ["Bezel Project · system art", "Bezel Project · game art",
                        "Orionsangel · console", "Orionsangel · plain console",
                        "Duimon · ultrawide 21:9", "Duimon · ultrawide 21:9 night"][index]
            }
            displayScopeSelected: function(scope) {}
            displaySettingSaved: function(field, value) {}
            translationOptInSelected: function(enabled) {
                testCase.requestedTranslationEnabled = enabled
            }
            ink: "#f4f7fb"
            muted: "#94a0b3"
            line: "#2b384b"
            accentCool: "#5de2d2"
        }
    }

    function test_installed_game_exposes_prominent_play_state() {
        const hero = createTemporaryObject(heroComponent, testCase)
        verify(hero)
        verify(hero.local)
        verify(hero.canLaunch)
        compare(hero.emulatorOptionCount, 2)
        compare(hero.emulatorLabelAt(1), "Mesen")
        verify(hero.implicitHeight > 100)
        const action = findChild(hero, "launchAction")
        verify(action)
        verify(action.visible)
        verify(action.positive)
        compare(action.background.color, "#237a4d")
    }

    function test_settings_accordion_hides_advanced_controls_but_not_play() {
        const hero = createTemporaryObject(heroComponent, testCase)
        verify(hero)
        const toggle = findChild(hero, "settingsAccordionButton")
        const translation = findChild(hero, "translationOptInButton")
        const display = findChild(hero, "displaySection")
        const emulator = findChild(hero, "emulatorSection")
        const mapping = findChild(hero, "controllerMappingButton")
        const manage = findChild(hero, "manageEmulatorsButton")
        const play = findChild(hero, "launchAction")
        verify(toggle && translation && display && emulator && mapping && manage && play)
        compare(hero.settingsExpanded, false)
        compare(translation.visible, false)
        compare(display.visible, false)
        compare(emulator.visible, false)
        compare(mapping.visible, false)
        compare(manage.visible, false)
        compare(play.visible, true)
        verify(play.y < toggle.y)
        wait(50)
        const collapsedHeight = hero.implicitHeight

        toggle.clicked()
        compare(hero.settingsExpanded, true)
        compare(translation.visible, true)
        compare(display.visible, true)
        compare(emulator.visible, true)
        compare(mapping.visible, true)
        compare(manage.visible, true)
        tryVerify(function() { return hero.implicitHeight > collapsedHeight + 100 })

        hero.gameId = "game-2"
        compare(hero.settingsExpanded, false)
        compare(play.visible, true)
        tryCompare(hero, "implicitHeight", collapsedHeight)

        toggle.clicked()
        compare(hero.settingsExpanded, true)
        toggle.clicked()
        compare(hero.settingsExpanded, false)
        compare(play.visible, true)
        tryCompare(hero, "implicitHeight", collapsedHeight)
    }

    function test_translation_requires_per_game_opt_in() {
        requestedTranslationEnabled = false
        const hero = createTemporaryObject(heroComponent, testCase)
        verify(hero)
        findChild(hero, "settingsAccordionButton").clicked()
        const optIn = findChild(hero, "translationOptInButton")
        verify(optIn)
        compare(optIn.text, "Translate this game: Off")
        optIn.clicked()
        compare(requestedTranslationEnabled, true)
        hero.translationOptedIn = true
        compare(optIn.text, "Translate this game: On")
        hero.selectedEmulatorOption = 1
        compare(optIn.visible, false)
    }

    function test_emulator_picker_splits_standalone_and_core_sections() {
        const hero = createTemporaryObject(heroComponent, testCase)
        verify(hero)
        compare(hero.standaloneIndices.length, 1)
        compare(hero.standaloneIndices[0], 1)
        compare(hero.retroarchIndices.length, 1)
        compare(hero.retroarchIndices[0], 0)
        verify(hero.hasStarredOption)
    }

    function test_display_section_appears_for_supported_adapters() {
        const hero = createTemporaryObject(heroComponent, testCase)
        verify(hero)
        const section = findChild(hero, "displaySection")
        verify(section)
        compare(hero.displaySectionAvailable, true)
        const fullscreen = findChild(hero, "displayFullscreenCombo")
        verify(fullscreen)
        compare(fullscreen.currentValue, "")
        hero.displayFullscreen = "true"
        hero.displayRevision++
        compare(fullscreen.currentValue, "true")
    }

    function test_display_accordion_hides_controls_without_losing_choices() {
        const hero = createTemporaryObject(heroComponent, testCase)
        verify(hero)
        const section = findChild(hero, "displaySection")
        const toggle = findChild(hero, "displayAccordionButton")
        const options = findChild(hero, "displayOptions")
        const fullscreen = findChild(hero, "displayFullscreenCombo")
        const emulator = findChild(hero, "emulatorSection")
        verify(section)
        verify(toggle)
        verify(options)
        verify(fullscreen)
        findChild(hero, "settingsAccordionButton").clicked()
        compare(toggle.width, section.width)
        compare(hero.displayExpanded, false)
        compare(options.visible, false)
        verify(findChild(emulator, "emulatorPicker"))
        wait(50)
        const collapsedHeight = hero.implicitHeight

        toggle.clicked()
        compare(hero.displayExpanded, true)
        compare(options.visible, true)
        tryVerify(function() { return hero.implicitHeight > collapsedHeight + 80 })
        hero.displayFullscreen = "true"
        hero.displayRevision++
        compare(fullscreen.currentValue, "true")

        toggle.clicked()
        compare(options.visible, false)
        compare(fullscreen.currentValue, "true")
        tryCompare(hero, "implicitHeight", collapsedHeight)
        toggle.clicked()
        compare(fullscreen.currentValue, "true")
    }

    function test_display_choices_use_the_available_pane_width() {
        const hero = createTemporaryObject(heroComponent, testCase)
        verify(hero)
        for (const name of ["displayFullscreenCombo", "displayShaderCombo",
                            "displayBezelCombo", "displaySaveStatesCombo"]) {
            const combo = findChild(hero, name)
            verify(combo)
            verify(combo.width >= hero.width - 40, name + " is too narrow")
        }
    }

    function test_narrow_display_choice_shows_full_value_below_native_combo() {
        const hero = createTemporaryObject(heroComponent, testCase, {
            width: 260,
            displayInheritedShaderLabel: "Inherit → RetroTube TV · bezel + ambient light (Koko-AIO)"
        })
        verify(hero)
        const shader = findChild(hero, "displayShaderCombo")
        verify(shader)
        const fullValue = findChild(hero, "displayShaderFullValue")
        verify(fullValue)
        compare(fullValue.text, shader.displayText)
        compare(fullValue.wrapMode, Text.WordWrap)
        verify(fullValue.text.length > 28)
    }

    function test_inherit_options_name_the_parent_value() {
        const hero = createTemporaryObject(heroComponent, testCase)
        verify(hero)
        const fullscreen = findChild(hero, "displayFullscreenCombo")
        const shader = findChild(hero, "displayShaderCombo")
        const bezel = findChild(hero, "displayBezelCombo")
        const states = findChild(hero, "displaySaveStatesCombo")
        compare(fullscreen.model[0].label, "Inherit → Off (RetroArch default)")
        compare(shader.model[0].label, "Inherit → Off (RetroArch default)")
        compare(bezel.model[0].label, "Inherit → System pack")
        compare(states.model[0].label, "Inherit → Save + resume")
        hero.displayInheritedShaderLabel = "Inherit → RetroTube TV"
        compare(shader.model[0].label, "Inherit → RetroTube TV")
    }

    function test_bezel_picker_lists_both_packs_and_console_variants() {
        const hero = createTemporaryObject(heroComponent, testCase)
        verify(hero)
        const bezel = findChild(hero, "displayBezelCombo")
        verify(bezel)
        compare(bezel.model.length, 8)
        compare(bezel.model[2].label, "Bezel Project · system art")
        compare(bezel.model[3].label, "Bezel Project · game art")
        compare(bezel.model[4].label, "Orionsangel · console")
        compare(bezel.model[5].label, "Orionsangel · plain console")
        compare(bezel.model[6].label, "Duimon · ultrawide 21:9")
        compare(bezel.model[7].label, "Duimon · ultrawide 21:9 night")
        hero.displayBezel = "ultrawide"
        hero.displayRevision++
        compare(bezel.currentValue, "ultrawide")
    }

    function test_bezel_popup_renders_long_choices_without_elision() {
        const hero = createTemporaryObject(heroComponent, testCase)
        verify(hero)
        findChild(hero, "settingsAccordionButton").clicked()
        findChild(hero, "displayAccordionButton").clicked()
        const bezel = findChild(hero, "displayBezelCombo")
        bezel.popup.open()
        verify(bezel.popup.visible)
        let found = false
        for (let attempt = 0; attempt < 50 && !found; ++attempt) {
            const rows = bezel.popup.contentItem.contentItem.children
            for (let i = 0; i < rows.length; ++i) {
                const content = rows[i].contentItem
                if (content && content.text === "Duimon · ultrawide 21:9 night") {
                    compare(content.wrapMode, Text.WordWrap)
                    compare(content.elide, Text.ElideNone)
                    found = true
                    break
                }
            }
            if (!found)
                wait(20)
        }
        verify(found)
    }

    function test_display_section_hides_for_unsupported_adapters() {
        const hero = createTemporaryObject(heroComponent, testCase, {
            displayFullscreenSupported: false,
            displayShaderSupported: false,
            displayBezelSupported: false,
            displaySaveStatesSupported: false
        })
        verify(hero)
        const section = findChild(hero, "displaySection")
        verify(section)
        compare(hero.displaySectionAvailable, false)
    }

    function test_display_effective_summary_names_what_inherit_resolves() {
        const hero = createTemporaryObject(heroComponent, testCase, {
            displayEffectiveSummary: "Effective: CRT RetroTube TV · Bezel System pack · States Save + resume"
        })
        verify(hero)
        const summary = findChild(hero, "displayEffectiveSummary")
        verify(summary)
        compare(summary.text, "Effective: CRT RetroTube TV · Bezel System pack · States Save + resume")
    }

    function test_display_effective_summary_empty_when_nothing_set() {
        const hero = createTemporaryObject(heroComponent, testCase)
        verify(hero)
        const summary = findChild(hero, "displayEffectiveSummary")
        verify(summary)
        compare(summary.text, "")
    }

    function test_emulator_picker_is_available_without_display_features() {
        const hero = createTemporaryObject(heroComponent, testCase, {
            displayFullscreenSupported: false,
            displayShaderSupported: false,
            displayBezelSupported: false,
            displaySaveStatesSupported: false
        })
        verify(hero)
        compare(hero.displaySectionAvailable, false)
        // The picker must not live inside displaySection: that column hides
        // when the auto-picked emulator has no display features (e.g. a
        // standalone SNES pick), which would leave no way to choose another.
        const display = findChild(hero, "displaySection")
        verify(display)
        compare(findChild(display, "emulatorPicker"), null)
        const section = findChild(hero, "emulatorSection")
        verify(section)
        compare(section.visible, false)
        findChild(hero, "settingsAccordionButton").clicked()
        compare(section.visible, true)
        compare(findChild(section, "emulatorPicker") === null, false)
        const picker = findChild(hero, "emulatorPicker")
        verify(picker)
        compare(picker.model.length, 2)
    }

    function test_merged_picker_lists_standalone_section_then_cores() {
        const hero = createTemporaryObject(heroComponent, testCase)
        verify(hero)
        const picker = findChild(hero, "emulatorPicker")
        verify(picker)
        compare(picker.model.length, 2)
        compare(picker.model[0].kind, "Standalone")
        compare(picker.model[0].label, "Mesen")
        compare(picker.model[1].kind, "RetroArch cores")
        compare(picker.model[1].label, "RetroArch · Mesen")
    }

    function test_merged_picker_syncs_with_selection() {
        const hero = createTemporaryObject(heroComponent, testCase, {
            selectedEmulatorOption: 0
        })
        verify(hero)
        const picker = findChild(hero, "emulatorPicker")
        verify(picker)
        // Option 0 is the RetroArch core, which sorts second in the merged
        // standalone-first model, so the picker row follows the selection.
        compare(picker.currentIndex, 1)
        compare(picker.displayText, "RetroArch · Mesen")
        hero.selectedEmulatorOption = 1
        compare(picker.currentIndex, 0)
        compare(picker.displayText, "Mesen")
    }

    function test_merged_picker_rebuilds_when_options_arrive() {
        const hero = createTemporaryObject(heroComponent, testCase, {
            emulatorOptionCount: 0
        })
        verify(hero)
        const picker = findChild(hero, "emulatorPicker")
        verify(picker)
        compare(picker.model.length, 0)
        hero.emulatorOptionCount = 2
        compare(picker.model.length, 2)
        compare(picker.model[0].label, "Mesen")
        compare(picker.model[1].label, "RetroArch · Mesen")
    }

    function test_merged_picker_popup_shows_all_labels() {
        const hero = createTemporaryObject(heroComponent, testCase)
        verify(hero)
        findChild(hero, "settingsAccordionButton").clicked()
        const picker = findChild(hero, "emulatorPicker")
        verify(picker)
        picker.popup.open()
        verify(picker.popup.visible)
        // The popup ListView instantiates rows asynchronously; poll until
        // both option labels (in any child order) have rendered text.
        let rows = ""
        for (let attempt = 0; attempt < 50; ++attempt) {
            const found = []
            const kids = picker.popup.contentItem.contentItem.children
            for (let i = 0; i < kids.length; ++i) {
                const label = kids[i].text
                if (label !== undefined && label.length > 0
                        && label !== "Standalone" && label !== "RetroArch cores"
                        && !found.includes(label))
                    found.push(label)
            }
            found.sort()
            if (found.join("|") === "Mesen|RetroArch · Mesen") {
                rows = found.join("|")
                break
            }
            wait(20)
        }
        compare(rows, "Mesen|RetroArch · Mesen")
    }

    function test_non_local_game_hides_play_section() {
        const hero = createTemporaryObject(heroComponent, testCase, { local: false })
        verify(hero)
        compare(hero.visible, false)
        compare(hero.height, 0)
    }

    function test_missing_emulator_exposes_install_state() {
        const hero = createTemporaryObject(heroComponent, testCase, {
            canLaunch: false,
            emulatorName: "No compatible emulator",
            emulatorOptionCount: 0,
            selectedEmulatorOption: -1
        })
        verify(hero)
        verify(hero.emulatorMissing)
        verify(!hero.prepareNeeded)
        verify(hero.local)
    }

    function test_preparable_archive_offers_prepare_instead_of_install() {
        const hero = createTemporaryObject(heroComponent, testCase, {
            canLaunch: false,
            emulatorName: "",
            emulatorOptionCount: 0,
            selectedEmulatorOption: -1,
            preparable: true
        })
        verify(hero)
        verify(!hero.emulatorMissing)
        verify(hero.prepareNeeded)
        let prepared = false
        hero.prepareRequested.connect(function() { prepared = true })
        const action = findChild(hero, "launchAction")
        verify(action)
        compare(action.text, "Prepare install")
        action.clicked()
        verify(prepared)
    }

    function test_prepare_in_progress_shows_busy_state() {
        const hero = createTemporaryObject(heroComponent, testCase, {
            canLaunch: false,
            emulatorName: "",
            emulatorOptionCount: 0,
            selectedEmulatorOption: -1,
            preparable: true,
            prepareBusy: true
        })
        verify(hero)
        verify(!hero.prepareNeeded)
        const action = findChild(hero, "launchAction")
        verify(action)
        compare(action.text, "Preparing install…")
        compare(action.enabled, false)
    }

    function test_launch_preparation_can_be_cancelled() {
        const hero = createTemporaryObject(heroComponent, testCase, {
            launchBusy: true
        })
        verify(hero)
        let cancelled = false
        hero.cancelLaunchRequested.connect(function() { cancelled = true })
        const action = findChild(hero, "launchAction")
        verify(action)
        compare(action.text, "Cancel preparation")
        verify(action.enabled)
        action.clicked()
        verify(cancelled)
        verify(hero.launchBusy)
    }

    function test_running_game_replaces_play_with_stop() {
        const hero = createTemporaryObject(heroComponent, testCase, {
            gameRunning: true,
            sessionTitle: "Seiken Densetsu 3"
        })
        verify(hero)
        const action = findChild(hero, "launchAction")
        verify(action)
        compare(action.text, "■  Stop emulator")
        verify(action.enabled)
        verify(!action.highlighted)
        verify(!action.positive)
        let stopped = false
        let played = false
        hero.stopEmulatorRequested.connect(function() { stopped = true })
        hero.playRequested.connect(function() { played = true })
        action.clicked()
        verify(stopped)
        verify(!played)
        hero.sessionStopping = true
        compare(action.text, "Stopping emulator…")
        verify(!action.enabled)
    }

    function test_missing_firmware_has_a_direct_setup_action() {
        const hero = createTemporaryObject(heroComponent, testCase, {
            canLaunch: false,
            emulatorName: "Ryubing (Ryujinx fork)",
            emulatorOptionCount: 1,
            selectedEmulatorOption: 0,
            firmwareMissingCount: 2,
            firmwareSetupLabel: "Set up emulator",
            launchStatus: "prod.keys and Switch firmware are required."
        })
        verify(hero)
        verify(hero.firmwareSetupNeeded)
        let setupRequested = false
        hero.firmwareSetupRequested.connect(function() { setupRequested = true })
        const action = findChild(hero, "launchAction")
        verify(action)
        compare(action.text, "Set up emulator")
        action.clicked()
        verify(setupRequested)
    }
}
