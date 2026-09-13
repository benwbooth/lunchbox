#[cxx_qt::bridge]
pub mod qobject {
    unsafe extern "C++" {
        include!("cxx-qt-lib/qstring.h");
        include!("cxx-qt-lib/qurl.h");
        type QString = cxx_qt_lib::QString;
        type QUrl = cxx_qt_lib::QUrl;
    }

    unsafe extern "RustQt" {
        #[qobject]
        #[qml_element]
        #[qproperty(bool, initialized)]
        #[qproperty(bool, onboarding_complete)]
        #[qproperty(bool, minimize_during_game)]
        #[qproperty(bool, busy)]
        #[qproperty(bool, password_saved)]
        #[qproperty(bool, connection_ok)]
        #[qproperty(QString, message)]
        #[qproperty(QString, state_database_path)]
        #[qproperty(bool, profile_busy)]
        #[qproperty(QString, profile_message)]
        #[qproperty(bool, profile_restore_ready)]
        #[qproperty(QString, profile_restore_summary)]
        #[qproperty(bool, profile_restart_required)]
        #[qproperty(QString, qbittorrent_host)]
        #[qproperty(i32, qbittorrent_port)]
        #[qproperty(bool, qbittorrent_use_https)]
        #[qproperty(QString, qbittorrent_username)]
        #[qproperty(QString, qbittorrent_password)]
        #[qproperty(QString, rom_directory)]
        #[qproperty(QString, qbittorrent_container_rom_directory)]
        #[qproperty(QString, torrent_library_directory)]
        #[qproperty(QString, qbittorrent_container_torrent_library_directory)]
        #[qproperty(QString, watched_torrent_directory)]
        #[qproperty(QString, watched_torrent_archive_directory)]
        #[qproperty(bool, download_entire_torrent)]
        #[qproperty(QString, file_link_mode)]
        #[qproperty(QString, seeding_policy)]
        #[qproperty(QString, preferred_region)]
        #[qproperty(i32, region_revision)]
        #[qproperty(QString, version_preference)]
        #[qproperty(i32, media_provider_revision)]
        #[qproperty(bool, shader_busy)]
        #[qproperty(i32, shader_progress)]
        #[qproperty(QString, shader_message)]
        #[qproperty(bool, shader_requires_confirmation)]
        #[qproperty(i32, shader_revision)]
        #[qproperty(bool, controller_enabled)]
        #[qproperty(bool, controller_automatic)]
        #[qproperty(bool, controller_calibrated_launch)]
        #[qproperty(bool, controller_remapping_available)]
        #[qproperty(QString, controller_output_target)]
        #[qproperty(bool, controller_busy)]
        #[qproperty(i32, controller_revision)]
        #[qproperty(QString, controller_status)]
        #[qproperty(bool, native_capture_busy)]
        #[qproperty(bool, fbneo_import_busy)]
        #[qproperty(bool, mame_inspection_busy)]
        #[qproperty(QString, mame_inspection_status)]
        #[qproperty(QString, mame_inspection_result)]
        #[qproperty(bool, fbneo_inspection_active)]
        #[qproperty(QString, fbneo_import_status)]
        #[qproperty(QString, fbneo_import_draft)]
        #[qproperty(bool, native_capture_ready)]
        #[qproperty(QString, native_capture_status)]
        #[qproperty(QString, native_capture_results)]
        #[qproperty(bool, controller_profile_editor_open)]
        #[qproperty(QString, controller_profile_editor_id)]
        #[qproperty(QString, controller_profile_editor_name)]
        #[qproperty(QString, controller_profile_editor_layout)]
        #[qproperty(QString, controller_profile_editor_target)]
        #[qproperty(QString, controller_profile_editor_status)]
        #[qproperty(i32, controller_profile_revision)]
        type SettingsModel = super::SettingsModelRust;

        #[qinvokable]
        fn initialize(self: Pin<&mut SettingsModel>);

        #[qinvokable]
        fn save(self: Pin<&mut SettingsModel>);

        #[qinvokable]
        fn test_connection(self: Pin<&mut SettingsModel>);

        #[qinvokable]
        fn invalidate_qbittorrent_test(self: Pin<&mut SettingsModel>);

        #[qinvokable]
        fn managed_native_download_directory(
            self: &SettingsModel,
            base_directory: QString,
        ) -> QString;

        #[qinvokable]
        fn managed_client_download_directory(
            self: &SettingsModel,
            base_directory: QString,
        ) -> QString;

        #[qinvokable]
        fn clear_password(self: Pin<&mut SettingsModel>);

        #[qinvokable]
        fn set_directory(self: Pin<&mut SettingsModel>, field: QString, url: QUrl);

        #[qinvokable]
        fn choose_native_directory(self: Pin<&mut SettingsModel>, field: QString);

        #[qinvokable]
        fn export_profile(self: Pin<&mut SettingsModel>, destination: QUrl);

        #[qinvokable]
        fn inspect_profile(self: Pin<&mut SettingsModel>, source: QUrl);

        #[qinvokable]
        fn stage_profile_restore(self: Pin<&mut SettingsModel>);

        #[qinvokable]
        fn cancel_profile_restore(self: Pin<&mut SettingsModel>);

        #[qinvokable]
        fn cancel_staged_profile_restore(self: Pin<&mut SettingsModel>);

        #[qinvokable]
        fn region_count(self: &SettingsModel) -> i32;

        #[qinvokable]
        fn region_at(self: &SettingsModel, index: i32) -> QString;

        #[qinvokable]
        fn move_region(self: Pin<&mut SettingsModel>, from: i32, to: i32);

        #[qinvokable]
        fn reset_region_priority(self: Pin<&mut SettingsModel>);

        #[qinvokable]
        fn media_provider_count(self: &SettingsModel) -> i32;

        #[qinvokable]
        fn media_provider_name_at(self: &SettingsModel, index: i32) -> QString;

        #[qinvokable]
        fn media_provider_description_at(self: &SettingsModel, index: i32) -> QString;

        #[qinvokable]
        fn move_media_provider(self: Pin<&mut SettingsModel>, from: i32, to: i32);

        #[qinvokable]
        fn reset_media_provider_priority(self: Pin<&mut SettingsModel>);

        #[qinvokable]
        fn refresh_retroarch_shaders(self: Pin<&mut SettingsModel>);

        #[qinvokable]
        fn install_retroarch_shaders(self: Pin<&mut SettingsModel>, replace_unmanaged: bool);

        #[qinvokable]
        fn cancel_retroarch_shaders(self: Pin<&mut SettingsModel>);

        #[qinvokable]
        fn shader_target_count(self: &SettingsModel) -> i32;

        #[qinvokable]
        fn shader_target_path_at(self: &SettingsModel, index: i32) -> QString;

        #[qinvokable]
        fn shader_target_detail_at(self: &SettingsModel, index: i32) -> QString;

        #[qinvokable]
        fn open_shader_target(self: Pin<&mut SettingsModel>, index: i32);

        #[qinvokable]
        fn refresh_controllers(self: Pin<&mut SettingsModel>);

        #[qinvokable]
        fn set_controller_mapping_enabled(self: Pin<&mut SettingsModel>, enabled: bool);

        #[qinvokable]
        fn set_controller_automatic_enabled(self: Pin<&mut SettingsModel>, enabled: bool);
        #[qinvokable]
        fn set_controller_calibrated_launch_enabled(self: Pin<&mut SettingsModel>, enabled: bool);
        #[qinvokable]
        fn configure_controller_routing(self: Pin<&mut SettingsModel>, enabled: bool);
        #[qinvokable]
        fn controller_layout_at(self: &SettingsModel, index: i32) -> QString;
        #[qinvokable]
        fn choose_controller_layout(self: Pin<&mut SettingsModel>, index: i32, layout: QString);
        #[qinvokable]
        fn controller_preference_at(self: &SettingsModel, system: QString) -> i32;
        #[qinvokable]
        fn choose_preferred_controller(self: Pin<&mut SettingsModel>, system: QString, index: i32);
        #[qinvokable]
        fn preferred_controller_profile(self: &SettingsModel, system: QString) -> QString;
        #[qinvokable]
        fn choose_preferred_controller_profile(
            self: Pin<&mut SettingsModel>,
            system: QString,
            profile: QString,
        );

        #[qinvokable]
        fn choose_default_controller_target(self: Pin<&mut SettingsModel>, target: QString);

        #[qinvokable]
        fn controller_count(self: &SettingsModel) -> i32;

        #[qinvokable]
        fn controller_name_at(self: &SettingsModel, index: i32) -> QString;

        #[qinvokable]
        fn controller_alias_at(self: &SettingsModel, index: i32) -> QString;

        #[qinvokable]
        fn rename_controller(self: Pin<&mut SettingsModel>, index: i32, name: QString);
        #[qinvokable]
        fn save_controller_name(
            self: Pin<&mut SettingsModel>,
            device: QString,
            name: QString,
        ) -> QString;

        #[qinvokable]
        fn controller_detail_at(self: &SettingsModel, index: i32) -> QString;

        #[qinvokable]
        fn controller_receives_input(self: &SettingsModel, index: i32, key: QString) -> bool;

        #[qinvokable]
        fn controller_key_at(self: &SettingsModel, index: i32) -> QString;
        #[qinvokable]
        fn controller_key_for_input(self: &SettingsModel, key: QString) -> QString;
        #[qinvokable]
        fn controller_catalog_json(self: &SettingsModel) -> QString;
        #[qinvokable]
        fn use_sdl3_controller_mapping(self: Pin<&mut SettingsModel>, device: QString) -> QString;
        #[qinvokable]
        fn controller_model_review(
            self: &SettingsModel,
            device: QString,
            query: QString,
        ) -> QString;
        #[qinvokable]
        fn save_controller_model(
            self: Pin<&mut SettingsModel>,
            device: QString,
            model: QString,
        ) -> QString;
        #[qinvokable]
        fn controller_coverage_json(self: &SettingsModel) -> QString;
        #[qinvokable]
        fn emulator_platform_locations_json(self: &SettingsModel, emulator: QString) -> QString;
        #[qinvokable]
        fn fbneo_controller_setups_json(self: &SettingsModel) -> QString;
        #[qinvokable]
        fn mame_controller_setups_json(self: &SettingsModel) -> QString;
        #[qinvokable]
        fn mame_arcade_layout(self: &SettingsModel) -> QString;
        #[qinvokable]
        fn relative_device_settings_json(self: &SettingsModel) -> QString;
        #[qinvokable]
        fn absolute_device_settings_json(self: &SettingsModel) -> QString;
        #[qinvokable]
        fn absolute_capture_command(self: Pin<&mut SettingsModel>, request: QString) -> QString;
        #[qinvokable]
        fn stage_absolute_device_settings(
            self: Pin<&mut SettingsModel>,
            configuration: QString,
        ) -> QString;
        #[qinvokable]
        fn absolute_calibration_preview_json(
            self: &SettingsModel,
            configuration: QString,
            raw_x: i32,
            raw_y: i32,
        ) -> QString;
        #[qinvokable]
        fn stage_relative_device_settings(
            self: Pin<&mut SettingsModel>,
            configuration: QString,
        ) -> QString;
        #[qinvokable]
        fn mame_dependency_discovery(self: &SettingsModel) -> bool;
        #[qinvokable]
        fn choose_mame_dependency_discovery(self: Pin<&mut SettingsModel>, enabled: bool);
        #[qinvokable]
        fn choose_mame_arcade_layout(self: Pin<&mut SettingsModel>, layout: QString) -> QString;
        #[qinvokable]
        fn duckstation_setups_json(self: &SettingsModel) -> QString;
        #[qinvokable]
        fn ppsspp_setups_json(self: &SettingsModel) -> QString;
        #[qinvokable]
        fn mgba_setups_json(self: &SettingsModel) -> QString;
        #[qinvokable]
        fn snes9x_setups_json(self: &SettingsModel) -> QString;
        #[qinvokable]
        fn review_snes9x_setups(self: &SettingsModel, configuration: QString) -> QString;
        #[qinvokable]
        fn stage_snes9x_setups(self: Pin<&mut SettingsModel>, configuration: QString) -> QString;
        #[qinvokable]
        fn fceux_setups_json(self: &SettingsModel) -> QString;
        #[qinvokable]
        fn sameboy_setups_json(self: &SettingsModel) -> QString;
        #[qinvokable]
        fn mednafen_setups_json(self: &SettingsModel) -> QString;
        #[qinvokable]
        fn flycast_native_setups_json(self: &SettingsModel) -> QString;
        #[qinvokable]
        fn pcsx2_setups_json(self: &SettingsModel) -> QString;
        #[qinvokable]
        fn rpcs3_setups_json(self: &SettingsModel) -> QString;
        #[qinvokable]
        fn melonds_setups_json(self: &SettingsModel) -> QString;
        #[qinvokable]
        fn review_melonds_setups(self: &SettingsModel, configuration: QString) -> QString;
        #[qinvokable]
        fn stage_melonds_setups(self: Pin<&mut SettingsModel>, configuration: QString) -> QString;
        #[qinvokable]
        fn review_rpcs3_setups(self: &SettingsModel, configuration: QString) -> QString;
        #[qinvokable]
        fn stage_rpcs3_setups(self: Pin<&mut SettingsModel>, configuration: QString) -> QString;
        #[qinvokable]
        fn review_pcsx2_setups(self: &SettingsModel, configuration: QString) -> QString;
        #[qinvokable]
        fn stage_pcsx2_setups(self: Pin<&mut SettingsModel>, configuration: QString) -> QString;
        #[qinvokable]
        fn review_flycast_native_setups(self: &SettingsModel, configuration: QString) -> QString;
        #[qinvokable]
        fn stage_flycast_native_setups(
            self: Pin<&mut SettingsModel>,
            configuration: QString,
        ) -> QString;
        #[qinvokable]
        fn mame_native_setups_json(self: &SettingsModel) -> QString;
        #[qinvokable]
        fn review_mame_native_setups(self: &SettingsModel, configuration: QString) -> QString;
        #[qinvokable]
        fn stage_mame_native_setups(
            self: Pin<&mut SettingsModel>,
            configuration: QString,
        ) -> QString;
        #[qinvokable]
        fn review_mednafen_setups(self: &SettingsModel, configuration: QString) -> QString;
        #[qinvokable]
        fn stage_mednafen_setups(self: Pin<&mut SettingsModel>, configuration: QString) -> QString;
        #[qinvokable]
        fn review_sameboy_setups(self: &SettingsModel, configuration: QString) -> QString;
        #[qinvokable]
        fn stage_sameboy_setups(self: Pin<&mut SettingsModel>, configuration: QString) -> QString;
        #[qinvokable]
        fn bsnes_setups_json(self: &SettingsModel) -> QString;
        #[qinvokable]
        fn review_bsnes_setups(self: &SettingsModel, configuration: QString) -> QString;
        #[qinvokable]
        fn stage_bsnes_setups(self: Pin<&mut SettingsModel>, configuration: QString) -> QString;
        #[qinvokable]
        fn stella_native_setups_json(self: &SettingsModel) -> QString;
        #[qinvokable]
        fn review_stella_native_setups(self: &SettingsModel, configuration: QString) -> QString;
        #[qinvokable]
        fn stage_stella_native_setups(
            self: Pin<&mut SettingsModel>,
            configuration: QString,
        ) -> QString;
        #[qinvokable]
        fn vice_native_setups_json(self: &SettingsModel) -> QString;
        #[qinvokable]
        fn review_vice_native_setups(self: &SettingsModel, configuration: QString) -> QString;
        #[qinvokable]
        fn stage_vice_native_setups(
            self: Pin<&mut SettingsModel>,
            configuration: QString,
        ) -> QString;
        #[qinvokable]
        fn hatari_native_setups_json(self: &SettingsModel) -> QString;
        #[qinvokable]
        fn review_hatari_native_setups(self: &SettingsModel, configuration: QString) -> QString;
        #[qinvokable]
        fn stage_hatari_native_setups(
            self: Pin<&mut SettingsModel>,
            configuration: QString,
        ) -> QString;
        #[qinvokable]
        fn desmume_native_setups_json(self: &SettingsModel) -> QString;
        #[qinvokable]
        fn review_desmume_native_setups(self: &SettingsModel, configuration: QString) -> QString;
        #[qinvokable]
        fn stage_desmume_native_setups(
            self: Pin<&mut SettingsModel>,
            configuration: QString,
        ) -> QString;
        #[qinvokable]
        fn openmsx_native_setups_json(self: &SettingsModel) -> QString;
        #[qinvokable]
        fn review_openmsx_native_setups(self: &SettingsModel, configuration: QString) -> QString;
        #[qinvokable]
        fn stage_openmsx_native_setups(
            self: Pin<&mut SettingsModel>,
            configuration: QString,
        ) -> QString;
        #[qinvokable]
        fn mesen2_native_setups_json(self: &SettingsModel) -> QString;
        #[qinvokable]
        fn review_mesen2_native_setups(self: &SettingsModel, configuration: QString) -> QString;
        #[qinvokable]
        fn stage_mesen2_native_setups(
            self: Pin<&mut SettingsModel>,
            configuration: QString,
        ) -> QString;
        #[qinvokable]
        fn blastem_native_setups_json(self: &SettingsModel) -> QString;
        #[qinvokable]
        fn review_blastem_native_setups(self: &SettingsModel, configuration: QString) -> QString;
        #[qinvokable]
        fn stage_blastem_native_setups(
            self: Pin<&mut SettingsModel>,
            configuration: QString,
        ) -> QString;
        #[qinvokable]
        fn xemu_native_setups_json(self: &SettingsModel) -> QString;
        #[qinvokable]
        fn review_xemu_native_setups(self: &SettingsModel, configuration: QString) -> QString;
        #[qinvokable]
        fn stage_xemu_native_setups(
            self: Pin<&mut SettingsModel>,
            configuration: QString,
        ) -> QString;
        #[qinvokable]
        fn scummvm_native_setups_json(self: &SettingsModel) -> QString;
        #[qinvokable]
        fn review_scummvm_native_setups(self: &SettingsModel, configuration: QString) -> QString;
        #[qinvokable]
        fn stage_scummvm_native_setups(
            self: Pin<&mut SettingsModel>,
            configuration: QString,
        ) -> QString;
        #[qinvokable]
        fn jgenesis_native_setups_json(self: &SettingsModel) -> QString;
        #[qinvokable]
        fn review_jgenesis_native_setups(self: &SettingsModel, configuration: QString) -> QString;
        #[qinvokable]
        fn stage_jgenesis_native_setups(
            self: Pin<&mut SettingsModel>,
            configuration: QString,
        ) -> QString;
        #[qinvokable]
        fn gopher64_native_setups_json(self: &SettingsModel) -> QString;
        #[qinvokable]
        fn review_gopher64_native_setups(self: &SettingsModel, configuration: QString) -> QString;
        #[qinvokable]
        fn stage_gopher64_native_setups(
            self: Pin<&mut SettingsModel>,
            configuration: QString,
        ) -> QString;
        #[qinvokable]
        fn b2_native_setups_json(self: &SettingsModel) -> QString;
        #[qinvokable]
        fn review_b2_native_setups(self: &SettingsModel, configuration: QString) -> QString;
        #[qinvokable]
        fn stage_b2_native_setups(self: Pin<&mut SettingsModel>, configuration: QString)
        -> QString;
        #[qinvokable]
        fn hypseus_native_setups_json(self: &SettingsModel) -> QString;
        #[qinvokable]
        fn review_hypseus_native_setups(self: &SettingsModel, configuration: QString) -> QString;
        #[qinvokable]
        fn stage_hypseus_native_setups(
            self: Pin<&mut SettingsModel>,
            configuration: QString,
        ) -> QString;
        #[qinvokable]
        fn rmg_native_setups_json(self: &SettingsModel) -> QString;
        #[qinvokable]
        fn review_rmg_native_setups(self: &SettingsModel, configuration: QString) -> QString;
        #[qinvokable]
        fn stage_rmg_native_setups(
            self: Pin<&mut SettingsModel>,
            configuration: QString,
        ) -> QString;
        #[qinvokable]
        fn simple64_native_setups_json(self: &SettingsModel) -> QString;
        #[qinvokable]
        fn review_simple64_native_setups(self: &SettingsModel, configuration: QString) -> QString;
        #[qinvokable]
        fn stage_simple64_native_setups(
            self: Pin<&mut SettingsModel>,
            configuration: QString,
        ) -> QString;
        #[qinvokable]
        fn review_fceux_setups(self: &SettingsModel, configuration: QString) -> QString;
        #[qinvokable]
        fn stage_fceux_setups(self: Pin<&mut SettingsModel>, configuration: QString) -> QString;
        #[qinvokable]
        fn dolphin_setups_json(self: &SettingsModel) -> QString;
        #[qinvokable]
        fn review_dolphin_setups(self: &SettingsModel, configuration: QString) -> QString;
        #[qinvokable]
        fn stage_dolphin_setups(self: Pin<&mut SettingsModel>, configuration: QString) -> QString;
        #[qinvokable]
        fn review_mgba_setups(self: &SettingsModel, configuration: QString) -> QString;
        #[qinvokable]
        fn stage_mgba_setups(self: Pin<&mut SettingsModel>, configuration: QString) -> QString;
        #[qinvokable]
        fn review_ppsspp_setups(self: &SettingsModel, configuration: QString) -> QString;
        #[qinvokable]
        fn stage_ppsspp_setups(self: Pin<&mut SettingsModel>, configuration: QString) -> QString;
        #[qinvokable]
        fn review_duckstation_setups(self: &SettingsModel, configuration: QString) -> QString;
        #[qinvokable]
        fn stage_duckstation_setups(
            self: Pin<&mut SettingsModel>,
            configuration: QString,
        ) -> QString;
        #[qinvokable]
        fn begin_mame_inspection(self: Pin<&mut SettingsModel>, request: QString) -> QString;
        #[qinvokable]
        fn cancel_mame_inspection(self: Pin<&mut SettingsModel>);
        #[qinvokable]
        fn apply_mame_inspection_json(self: &SettingsModel, configuration: QString) -> QString;
        #[qinvokable]
        fn mame_inspection_request_json(
            self: &SettingsModel,
            configuration: QString,
            runtime: QString,
            discovery_roots: QString,
        ) -> QString;
        #[qinvokable]
        fn mame_assignment_review_json(self: &SettingsModel, configuration: QString) -> QString;
        #[qinvokable]
        fn mame_relative_plan_preview_json(
            self: &SettingsModel,
            configuration: QString,
            assignments: QString,
        ) -> QString;
        #[qinvokable]
        fn mame_digital_channels_json(self: &SettingsModel) -> QString;
        #[qinvokable]
        fn mame_controller_setup_json(self: &SettingsModel, key: QString) -> QString;
        #[qinvokable]
        fn save_mame_controller_setup(
            self: Pin<&mut SettingsModel>,
            configuration: QString,
        ) -> QString;
        #[qinvokable]
        fn remove_mame_controller_setup(self: Pin<&mut SettingsModel>, key: QString) -> QString;
        #[qinvokable]
        fn fbneo_source_controllers_json(self: &SettingsModel) -> QString;
        #[qinvokable]
        fn fbneo_device_choices_json(self: &SettingsModel, context: QString) -> QString;
        #[qinvokable]
        fn begin_fbneo_inspection_import(
            self: Pin<&mut SettingsModel>,
            request: QString,
            report: QString,
            context: QString,
        ) -> QString;
        #[qinvokable]
        fn begin_fbneo_inspection(
            self: Pin<&mut SettingsModel>,
            request: QString,
            context: QString,
        ) -> QString;
        #[qinvokable]
        fn cancel_fbneo_inspection(self: Pin<&mut SettingsModel>);
        #[qinvokable]
        fn fbneo_assignment_review_json(self: &SettingsModel, configuration: QString) -> QString;
        #[qinvokable]
        fn fbneo_controller_setup_json(self: &SettingsModel, key: QString) -> QString;
        #[qinvokable]
        fn save_fbneo_controller_setup(
            self: Pin<&mut SettingsModel>,
            configuration: QString,
        ) -> QString;
        #[qinvokable]
        fn remove_fbneo_controller_setup(self: Pin<&mut SettingsModel>, key: QString) -> QString;
        #[qinvokable]
        fn choose_controller_launch_mode(
            self: Pin<&mut SettingsModel>,
            core: QString,
            platform: QString,
            profile: QString,
        ) -> QString;
        #[qinvokable]
        fn controller_diagram(self: &SettingsModel, layout: QString, active: QString) -> QString;
        #[qinvokable]
        fn controller_calibration_json(self: &SettingsModel, device: QString) -> QString;
        #[qinvokable]
        fn saved_controller_choices_json(self: &SettingsModel) -> QString;
        #[qinvokable]
        fn begin_native_controller_capture(
            self: Pin<&mut SettingsModel>,
            device: QString,
            runtime_key: QString,
        ) -> QString;
        #[qinvokable]
        fn finish_native_controller_capture(self: Pin<&mut SettingsModel>) -> QString;
        #[qinvokable]
        fn cancel_native_controller_capture(self: Pin<&mut SettingsModel>);
        #[qinvokable]
        fn native_controller_calibration_json(self: &SettingsModel, device: QString) -> QString;
        #[qinvokable]
        fn scoped_native_controller_calibration_json(
            self: &SettingsModel,
            device: QString,
            runtime_key: QString,
        ) -> QString;
        #[qinvokable]
        fn clear_scoped_native_controller_calibration(
            self: Pin<&mut SettingsModel>,
            device: QString,
            runtime_key: QString,
        );
        #[qinvokable]
        fn native_controller_runtime_json(self: &SettingsModel) -> QString;
        #[qinvokable]
        fn native_controller_runtimes_json(self: &SettingsModel) -> QString;
        #[qinvokable]
        fn native_controller_player_layout_json(
            self: &SettingsModel,
            player: QString,
            core: QString,
            emulator: QString,
        ) -> QString;
        #[qinvokable]
        fn native_controller_emulators_json(self: &SettingsModel) -> QString;
        #[qinvokable]
        fn save_native_controller_runtime(
            self: Pin<&mut SettingsModel>,
            configuration: QString,
            expected: QString,
        ) -> QString;
        #[qinvokable]
        fn clear_native_controller_calibration(self: Pin<&mut SettingsModel>, device: QString);
        #[qinvokable]
        fn remove_native_controller_runtime(
            self: Pin<&mut SettingsModel>,
            expected: QString,
        ) -> QString;
        #[qinvokable]
        fn save_native_controller_gesture(
            self: Pin<&mut SettingsModel>,
            device: QString,
            control: QString,
            selected: i32,
        ) -> QString;
        #[qinvokable]
        fn validate_controller_capture(
            self: &SettingsModel,
            layout: QString,
            control: QString,
            binding: QString,
        ) -> QString;
        #[qinvokable]
        fn save_controller_calibration_if_unchanged(
            self: Pin<&mut SettingsModel>,
            device: QString,
            layout: QString,
            bindings: QString,
            expected: QString,
        ) -> QString;
        #[qinvokable]
        fn save_controller_calibration(
            self: Pin<&mut SettingsModel>,
            device: QString,
            layout: QString,
            bindings: QString,
        ) -> QString;
        #[qinvokable]
        fn controller_mapping_preview(
            self: &SettingsModel,
            layout: QString,
            bindings: QString,
            profile: QString,
        ) -> QString;
        #[qinvokable]
        fn controller_layout_mapping(
            self: &SettingsModel,
            source: QString,
            destination: QString,
        ) -> QString;
        #[qinvokable]
        fn guided_controller_preview(
            self: &SettingsModel,
            device: QString,
            profile: QString,
            choices: QString,
        ) -> QString;
        #[qinvokable]
        fn controller_target_profile(
            self: &SettingsModel,
            emulator: QString,
            platform: QString,
        ) -> QString;
        #[qinvokable]
        fn save_controller_target_profile(
            self: Pin<&mut SettingsModel>,
            emulator: QString,
            platform: QString,
            profile: QString,
        ) -> QString;
        #[qinvokable]
        fn save_guided_controller_mapping(
            self: Pin<&mut SettingsModel>,
            device: QString,
            profile: QString,
            choices: QString,
            expected: QString,
        ) -> QString;
        #[qinvokable]
        fn persist_controller_calibration(
            self: Pin<&mut SettingsModel>,
            device: QString,
        ) -> QString;

        #[qinvokable]
        fn controller_action_at(self: &SettingsModel, index: i32) -> QString;

        #[qinvokable]
        fn controller_player_order_json(self: &SettingsModel) -> QString;
        #[qinvokable]
        fn save_controller_player_order(self: Pin<&mut SettingsModel>, players: QString)
        -> QString;

        #[qinvokable]
        fn controller_profile_at(self: &SettingsModel, index: i32) -> QString;

        #[qinvokable]
        fn controller_target_at(self: &SettingsModel, index: i32) -> QString;

        #[qinvokable]
        fn move_controller(self: Pin<&mut SettingsModel>, index: i32, direction: i32);

        #[qinvokable]
        fn choose_controller_action(self: Pin<&mut SettingsModel>, index: i32, action: QString);

        #[qinvokable]
        fn choose_controller_profile(self: Pin<&mut SettingsModel>, index: i32, profile: QString);

        #[qinvokable]
        fn choose_controller_target(self: Pin<&mut SettingsModel>, index: i32, target: QString);

        #[qinvokable]
        fn controller_profile_count(self: &SettingsModel) -> i32;

        #[qinvokable]
        fn controller_profile_id_at(self: &SettingsModel, index: i32) -> QString;

        #[qinvokable]
        fn controller_profile_name_at(self: &SettingsModel, index: i32) -> QString;

        #[qinvokable]
        fn controller_target_count(self: &SettingsModel) -> i32;

        #[qinvokable]
        fn controller_target_id_at(self: &SettingsModel, index: i32) -> QString;

        #[qinvokable]
        fn controller_target_name_at(self: &SettingsModel, index: i32) -> QString;

        #[qinvokable]
        fn custom_controller_profile_count(self: &SettingsModel) -> i32;

        #[qinvokable]
        fn custom_controller_profile_id_at(self: &SettingsModel, index: i32) -> QString;

        #[qinvokable]
        fn custom_controller_profile_name_at(self: &SettingsModel, index: i32) -> QString;

        #[qinvokable]
        fn custom_controller_profile_detail_at(self: &SettingsModel, index: i32) -> QString;

        #[qinvokable]
        fn create_controller_profile(self: Pin<&mut SettingsModel>);

        #[qinvokable]
        fn edit_controller_profile(self: Pin<&mut SettingsModel>, index: i32);

        #[qinvokable]
        fn delete_controller_profile(self: Pin<&mut SettingsModel>, index: i32);

        #[qinvokable]
        fn cancel_controller_profile_edit(self: Pin<&mut SettingsModel>);

        #[qinvokable]
        fn save_controller_profile(self: Pin<&mut SettingsModel>);

        #[qinvokable]
        fn select_controller_profile_target(self: Pin<&mut SettingsModel>, target: QString);

        #[qinvokable]
        fn controller_profile_button_count(self: &SettingsModel) -> i32;

        #[qinvokable]
        fn controller_profile_button_id_at(self: &SettingsModel, index: i32) -> QString;

        #[qinvokable]
        fn controller_profile_button_name_at(self: &SettingsModel, index: i32) -> QString;

        #[qinvokable]
        fn controller_profile_button_label(self: &SettingsModel, button: QString) -> QString;

        #[qinvokable]
        fn controller_profile_selected_source(self: &SettingsModel) -> QString;

        #[qinvokable]
        fn choose_controller_profile_source(self: Pin<&mut SettingsModel>, source: QString);

        #[qinvokable]
        fn controller_profile_mapping_count(self: &SettingsModel) -> i32;

        #[qinvokable]
        fn controller_profile_mapping_source_at(self: &SettingsModel, index: i32) -> QString;

        #[qinvokable]
        fn controller_profile_mapping_target_at(self: &SettingsModel, index: i32) -> QString;

        #[qinvokable]
        fn apply_two_button_controller_preset(self: Pin<&mut SettingsModel>);

        #[qinvokable]
        fn clear_controller_profile_mappings(self: Pin<&mut SettingsModel>);
    }

    impl cxx_qt::Threading for SettingsModel {}
}

use std::path::PathBuf;
use std::pin::Pin;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

use cxx_qt::{CxxQtType, Threading};
use cxx_qt_lib::{QString, QUrl};

use crate::controllers::{self, ControllerDevice, ControllerInventory};
use crate::profile_backup;
use crate::qbittorrent;
use crate::retroarch_shaders::{self, ShaderInstallSummary, ShaderInventory, ShaderProgress};
use crate::settings::{
    self, AppSettings, ControllerButtonMapping, ControllerCustomProfile, ControllerMappingSettings,
    SettingsStore,
};

pub struct SettingsModelRust {
    initialized: bool,
    onboarding_complete: bool,
    minimize_during_game: bool,
    busy: bool,
    password_saved: bool,
    connection_ok: bool,
    credential_generation: u64,
    message: QString,
    state_database_path: QString,
    profile_busy: bool,
    profile_message: QString,
    profile_restore_ready: bool,
    profile_restore_summary: QString,
    profile_restart_required: bool,
    profile_restore_source: Option<PathBuf>,
    qbittorrent_host: QString,
    qbittorrent_port: i32,
    qbittorrent_use_https: bool,
    qbittorrent_username: QString,
    qbittorrent_password: QString,
    rom_directory: QString,
    qbittorrent_container_rom_directory: QString,
    torrent_library_directory: QString,
    qbittorrent_container_torrent_library_directory: QString,
    watched_torrent_directory: QString,
    watched_torrent_archive_directory: QString,
    download_entire_torrent: bool,
    file_link_mode: QString,
    seeding_policy: QString,
    preferred_region: QString,
    region_priority: Vec<String>,
    region_revision: i32,
    version_preference: QString,
    media_provider_priority: Vec<String>,
    media_provider_revision: i32,
    shader_busy: bool,
    shader_progress: i32,
    shader_message: QString,
    shader_requires_confirmation: bool,
    shader_revision: i32,
    shader_inventory: ShaderInventory,
    shader_generation: u64,
    shader_cancel: Option<Arc<AtomicBool>>,
    controller_enabled: bool,
    controller_automatic: bool,
    controller_calibrated_launch: bool,
    controller_remapping_available: bool,
    controller_output_target: QString,
    controller_busy: bool,
    controller_revision: i32,
    controller_status: QString,
    native_capture_busy: bool,
    fbneo_import_busy: bool,
    mame_inspection_busy: bool,
    mame_inspection_status: QString,
    mame_inspection_result: QString,
    mame_inspection_cancel: Option<Arc<AtomicBool>>,
    mame_inspection_receipt: Option<serde_json::Value>,
    fbneo_inspection_active: bool,
    fbneo_inspection_cancel: Option<Arc<AtomicBool>>,
    fbneo_import_status: QString,
    fbneo_import_draft: QString,
    native_capture_ready: bool,
    native_capture_status: QString,
    native_capture_results: QString,
    native_capture_generation: u64,
    native_capture_cancel: Option<Arc<AtomicBool>>,
    native_capture_device: Option<String>,
    native_capture_runtime: Option<[String; 2]>,
    native_capture_report: Option<crate::controller_bizhawk::CapturedGesture>,
    #[cfg(target_os = "linux")]
    native_capture: Option<crate::controller_bizhawk::NativeGestureCapture>,
    #[cfg(all(target_os = "linux", target_pointer_width = "64"))]
    absolute_capture: Option<crate::controller_axis::absolute_session::AbsoluteCalibrationSession>,
    controller_mapping: ControllerMappingSettings,
    controller_inventory: Option<ControllerInventory>,
    controller_profile_editor_open: bool,
    controller_profile_editor_id: QString,
    controller_profile_editor_name: QString,
    controller_profile_editor_layout: QString,
    controller_profile_editor_target: QString,
    controller_profile_editor_status: QString,
    controller_profile_revision: i32,
    controller_profile_editor_mappings: Vec<ControllerButtonMapping>,
    controller_profile_pending_controller_id: Option<String>,
}

impl Default for SettingsModelRust {
    fn default() -> Self {
        Self {
            initialized: false,
            onboarding_complete: false,
            minimize_during_game: false,
            busy: false,
            password_saved: false,
            connection_ok: false,
            credential_generation: 0,
            message: QString::from("Loading settings…"),
            state_database_path: QString::default(),
            profile_busy: false,
            profile_message: QString::from(
                "Create a portable backup of saved collection state and preferences.",
            ),
            profile_restore_ready: false,
            profile_restore_summary: QString::default(),
            profile_restart_required: false,
            profile_restore_source: None,
            qbittorrent_host: QString::from("127.0.0.1"),
            qbittorrent_port: 8080,
            qbittorrent_use_https: false,
            qbittorrent_username: QString::default(),
            qbittorrent_password: QString::default(),
            rom_directory: QString::default(),
            qbittorrent_container_rom_directory: QString::default(),
            torrent_library_directory: QString::default(),
            qbittorrent_container_torrent_library_directory: QString::default(),
            watched_torrent_directory: QString::default(),
            watched_torrent_archive_directory: QString::default(),
            download_entire_torrent: false,
            file_link_mode: QString::from("symlink"),
            seeding_policy: QString::from("follow_client"),
            preferred_region: QString::from("USA"),
            region_priority: crate::region_priority::default_region_priority(),
            region_revision: 0,
            version_preference: QString::from("latest"),
            media_provider_priority: crate::media::default_provider_priority(),
            media_provider_revision: 0,
            shader_busy: false,
            shader_progress: 0,
            shader_message: QString::from(
                "Open Settings to inspect this computer's RetroArch shader folders.",
            ),
            shader_requires_confirmation: false,
            shader_revision: 0,
            shader_inventory: ShaderInventory::default(),
            shader_generation: 0,
            shader_cancel: None,
            controller_enabled: false,
            controller_automatic: false,
            controller_calibrated_launch: true,
            controller_remapping_available: false,
            controller_output_target: QString::from("xb360"),
            controller_busy: false,
            controller_revision: 0,
            controller_status: QString::from("Open controller settings to scan this computer."),
            native_capture_busy: false,
            fbneo_import_busy: false,
            mame_inspection_busy: false,
            mame_inspection_status: QString::default(),
            mame_inspection_result: QString::default(),
            mame_inspection_cancel: None,
            mame_inspection_receipt: None,
            fbneo_inspection_active: false,
            fbneo_inspection_cancel: None,
            fbneo_import_status: QString::default(),
            fbneo_import_draft: QString::default(),
            native_capture_ready: false,
            native_capture_status: QString::default(),
            native_capture_results: QString::from("[]"),
            native_capture_generation: 0,
            native_capture_cancel: None,
            native_capture_device: None,
            native_capture_runtime: None,
            native_capture_report: None,
            #[cfg(target_os = "linux")]
            native_capture: None,
            #[cfg(all(target_os = "linux", target_pointer_width = "64"))]
            absolute_capture: None,
            controller_mapping: ControllerMappingSettings::default(),
            controller_inventory: None,
            controller_profile_editor_open: false,
            controller_profile_editor_id: QString::default(),
            controller_profile_editor_name: QString::from("Custom profile"),
            controller_profile_editor_layout: QString::from("xbox"),
            controller_profile_editor_target: QString::from("South"),
            controller_profile_editor_status: QString::default(),
            controller_profile_revision: 0,
            controller_profile_editor_mappings: Vec::new(),
            controller_profile_pending_controller_id: None,
        }
    }
}

fn qstring(value: impl AsRef<str>) -> QString {
    QString::from(value.as_ref())
}

fn shader_ui_probe_enabled() -> bool {
    std::env::args_os().any(|argument| argument == "--retroarch-shader-ui-probe")
}

fn shader_probe_target() -> Option<PathBuf> {
    shader_ui_probe_enabled()
        .then(|| crate::catalog::requested_path("--shader-target", "LUNCHBOX_SHADER_TARGET"))
        .flatten()
}

fn shader_probe_archive_directory() -> Option<PathBuf> {
    shader_ui_probe_enabled()
        .then(|| {
            crate::catalog::requested_path(
                "--shader-archive-directory",
                "LUNCHBOX_SHADER_ARCHIVE_DIRECTORY",
            )
        })
        .flatten()
}

fn format_shader_bytes(bytes: u64) -> String {
    if bytes >= 1024 * 1024 {
        format!("{:.1} MiB", bytes as f64 / (1024.0 * 1024.0))
    } else if bytes >= 1024 {
        format!("{:.1} KiB", bytes as f64 / 1024.0)
    } else {
        format!("{bytes} B")
    }
}

impl qobject::SettingsModel {
    pub fn initialize(mut self: Pin<&mut Self>) {
        if *self.as_ref().initialized() || *self.as_ref().busy() {
            return;
        }
        self.as_mut().set_busy(true);
        self.as_mut().set_message(qstring("Loading settings…"));
        let qt_thread = self.as_ref().qt_thread();
        let spawn_result = std::thread::Builder::new()
            .name("lunchbox-settings-load".into())
            .spawn(move || {
                let loaded = load_settings().map_err(|error| {
                    eprintln!("LUNCHBOX_SETTINGS_LOAD_FAILED error={error:#}");
                    error.to_string()
                });
                let _ = qt_thread.queue(move |mut model| {
                    model.as_mut().finish_initialize(loaded);
                });
            });
        if let Err(error) = spawn_result {
            self.as_mut().set_busy(false);
            self.as_mut()
                .set_message(qstring(format!("Could not start settings worker: {error}")));
        }
    }

    fn finish_initialize(
        mut self: Pin<&mut Self>,
        loaded: Result<
            (
                AppSettings,
                Option<settings::QbittorrentConnectionTest>,
                PathBuf,
                Option<profile_backup::ProfileSummary>,
            ),
            String,
        >,
    ) {
        self.as_mut().set_busy(false);
        match loaded {
            Ok((settings, connection_test, path, restored_profile)) => {
                self.as_mut().apply_settings(settings);
                self.as_mut()
                    .set_state_database_path(qstring(path.to_string_lossy()));
                self.as_mut().set_initialized(true);
                if let Some(summary) = restored_profile {
                    self.as_mut().set_profile_message(qstring(format!(
                        "Profile restored safely at startup: {}. Credentials stayed in this computer's credential store.",
                        summary.description()
                    )));
                }
                self.as_mut()
                    .set_message(qstring("Settings loaded. Checking saved credentials…"));
                self.as_mut().load_saved_credential_status(connection_test);
            }
            Err(error) => self
                .as_mut()
                .set_message(qstring(format!("Could not load settings: {error}"))),
        }
    }

    fn load_saved_credential_status(
        mut self: Pin<&mut Self>,
        connection_test: Option<settings::QbittorrentConnectionTest>,
    ) {
        self.as_mut().rust_mut().credential_generation =
            self.as_ref().rust().credential_generation.wrapping_add(1);
        let generation = self.as_ref().rust().credential_generation;
        let qt_thread = self.as_ref().qt_thread();
        let spawn_result = std::thread::Builder::new()
            .name("lunchbox-settings-credentials".into())
            .spawn(move || {
                let credential_status = settings::load_password()
                    .map(|password| password.is_some())
                    .map_err(|error| {
                        eprintln!("LUNCHBOX_CREDENTIAL_STORE_UNAVAILABLE error={error:#}");
                        error.to_string()
                    });
                let _ = qt_thread.queue(move |mut model| {
                    model.as_mut().finish_saved_credential_status(
                        generation,
                        connection_test,
                        credential_status,
                    );
                });
            });
        if let Err(error) = spawn_result {
            eprintln!("LUNCHBOX_CREDENTIAL_WORKER_FAILED error={error}");
        }
    }

    fn finish_saved_credential_status(
        mut self: Pin<&mut Self>,
        generation: u64,
        connection_test: Option<settings::QbittorrentConnectionTest>,
        credential_status: Result<bool, String>,
    ) {
        if generation != self.as_ref().rust().credential_generation {
            return;
        }
        match credential_status {
            Ok(password_saved) => {
                let connection_ok = connection_test.as_ref().is_some_and(|tested| {
                    self.as_ref()
                        .settings_snapshot()
                        .is_ok_and(|current| tested.matches(&current, password_saved))
                });
                let tested_version = connection_test
                    .filter(|_| connection_ok)
                    .map(|tested| tested.version);
                self.as_mut().set_password_saved(password_saved);
                self.as_mut().set_connection_ok(connection_ok);
                self.as_mut().set_message(qstring(if let Some(version) = tested_version {
                    format!("qBittorrent {version} connection is verified and ready.")
                } else if password_saved {
                    "Settings loaded; the qBittorrent password is in your system credential store."
                        .to_owned()
                } else {
                    "Settings loaded. Configure qBittorrent to enable Minerva downloads."
                        .to_owned()
                }));
            }
            Err(error) => self.as_mut().set_message(qstring(format!(
                "Settings loaded, but the operating-system credential store is unavailable: {error}"
            ))),
        }
    }

    pub fn save(mut self: Pin<&mut Self>) {
        if *self.as_ref().busy() {
            return;
        }
        let settings = match self.as_ref().settings_snapshot() {
            Ok(settings) => settings,
            Err(error) => {
                self.as_mut().set_connection_ok(false);
                self.as_mut().set_message(qstring(error));
                return;
            }
        };
        let password = self.as_ref().qbittorrent_password().to_string();
        if !password.is_empty() {
            self.as_mut().rust_mut().credential_generation =
                self.as_ref().rust().credential_generation.wrapping_add(1);
        }
        let connection_ok = *self.as_ref().connection_ok();
        self.as_mut().set_busy(true);
        self.as_mut().set_message(qstring("Saving settings…"));
        let qt_thread = self.as_ref().qt_thread();
        let spawn_result = std::thread::Builder::new()
            .name("lunchbox-settings-save".into())
            .spawn(move || {
                let password_changed = !password.is_empty();
                let saved = save_settings(&settings, &password, connection_ok)
                    .map_err(|error| error.to_string());
                let _ = qt_thread.queue(move |mut model| {
                    model.as_mut().finish_save(saved, password_changed);
                });
            });
        if let Err(error) = spawn_result {
            self.as_mut().set_busy(false);
            self.as_mut()
                .set_message(qstring(format!("Could not start settings worker: {error}")));
        }
    }

    fn finish_save(
        mut self: Pin<&mut Self>,
        saved: Result<PathBuf, String>,
        password_changed: bool,
    ) {
        self.as_mut().set_busy(false);
        match saved {
            Ok(path) => {
                if password_changed {
                    self.as_mut().set_password_saved(true);
                    self.as_mut().set_qbittorrent_password(QString::default());
                }
                self.as_mut()
                    .set_state_database_path(qstring(path.to_string_lossy()));
                let connection_ok = *self.as_ref().connection_ok();
                self.as_mut().set_message(qstring(if connection_ok {
                    "Settings saved. The verified qBittorrent connection is ready."
                } else {
                    "Settings saved. Test qBittorrent before downloading."
                }));
            }
            Err(error) => self
                .as_mut()
                .set_message(qstring(format!("Could not save settings: {error}"))),
        }
    }

    pub fn test_connection(mut self: Pin<&mut Self>) {
        if *self.as_ref().busy() {
            return;
        }
        self.as_mut().rust_mut().credential_generation =
            self.as_ref().rust().credential_generation.wrapping_add(1);
        let settings = match self.as_ref().settings_snapshot() {
            Ok(settings) => settings,
            Err(error) => {
                self.as_mut().set_connection_ok(false);
                self.as_mut().set_message(qstring(error));
                return;
            }
        };
        let entered_password = self.as_ref().qbittorrent_password().to_string();
        self.as_mut().set_busy(true);
        self.as_mut().set_connection_ok(false);
        self.as_mut()
            .set_message(qstring("Testing qBittorrent connection…"));
        let qt_thread = self.as_ref().qt_thread();
        let spawn_result = std::thread::Builder::new()
            .name("lunchbox-qbittorrent-test".into())
            .spawn(move || {
                let tested = effective_password(entered_password)
                    .and_then(|password| test_and_record_connection(&settings, &password))
                    .map_err(|error| error.to_string());
                let _ = qt_thread.queue(move |mut model| {
                    model.as_mut().finish_test(tested);
                });
            });
        if let Err(error) = spawn_result {
            self.as_mut().set_busy(false);
            self.as_mut().set_message(qstring(format!(
                "Could not start connection worker: {error}"
            )));
        }
    }

    fn finish_test(
        mut self: Pin<&mut Self>,
        tested: Result<qbittorrent::QbittorrentConnectionDetails, String>,
    ) {
        self.as_mut().set_busy(false);
        match tested {
            Ok(details) => {
                self.as_mut().set_connection_ok(true);
                let detected_path = details.default_save_path.trim();
                if self
                    .as_ref()
                    .qbittorrent_container_torrent_library_directory()
                    .is_empty()
                    && !detected_path.is_empty()
                {
                    self.as_mut()
                        .set_qbittorrent_container_torrent_library_directory(qstring(
                            detected_path,
                        ));
                }
                if self.as_ref().torrent_library_directory().is_empty()
                    && !detected_path.is_empty()
                    && std::path::Path::new(detected_path).is_dir()
                {
                    self.as_mut()
                        .set_torrent_library_directory(qstring(detected_path));
                }
                let message = if !detected_path.is_empty()
                    && self.as_ref().torrent_library_directory().is_empty()
                {
                    format!(
                        "Connected to qBittorrent {}. Its download path is {detected_path}; choose the matching folder on this computer below, then save.",
                        details.version,
                    )
                } else {
                    format!("Connected to qBittorrent {}.", details.version)
                };
                self.as_mut().set_message(qstring(message));
            }
            Err(error) => {
                self.as_mut().set_connection_ok(false);
                self.as_mut()
                    .set_message(qstring(format!("Connection failed: {error}")));
            }
        }
    }

    pub fn invalidate_qbittorrent_test(mut self: Pin<&mut Self>) {
        if *self.as_ref().connection_ok() {
            self.as_mut().set_connection_ok(false);
            self.as_mut().set_message(qstring(
                "qBittorrent connection details changed. Test the new values before downloading.",
            ));
        }
    }

    pub fn managed_native_download_directory(&self, base_directory: QString) -> QString {
        let base_directory = base_directory.to_string();
        if base_directory.trim().is_empty() {
            return QString::default();
        }
        qstring(
            qbittorrent::managed_native_download_path(std::path::Path::new(&base_directory))
                .to_string_lossy(),
        )
    }

    pub fn managed_client_download_directory(&self, base_directory: QString) -> QString {
        let base_directory = base_directory.to_string();
        if base_directory.trim().is_empty() {
            return QString::default();
        }
        qstring(qbittorrent::managed_client_save_path(&base_directory))
    }

    pub fn clear_password(mut self: Pin<&mut Self>) {
        if *self.as_ref().busy() {
            return;
        }
        self.as_mut().rust_mut().credential_generation =
            self.as_ref().rust().credential_generation.wrapping_add(1);
        self.as_mut().set_busy(true);
        self.as_mut().set_connection_ok(false);
        let qt_thread = self.as_ref().qt_thread();
        let spawn_result = std::thread::Builder::new()
            .name("lunchbox-password-clear".into())
            .spawn(move || {
                let cleared = settings::save_password("")
                    .and_then(|()| {
                        SettingsStore::open_default()?.clear_qbittorrent_connection_test()
                    })
                    .map_err(|error| error.to_string());
                let _ = qt_thread.queue(move |mut model| {
                    model.as_mut().set_busy(false);
                    match cleared {
                        Ok(()) => {
                            model.as_mut().set_password_saved(false);
                            model.as_mut().set_qbittorrent_password(QString::default());
                            model
                                .as_mut()
                                .set_message(qstring("Saved qBittorrent password removed."));
                        }
                        Err(error) => model
                            .as_mut()
                            .set_message(qstring(format!("Could not remove password: {error}"))),
                    }
                });
            });
        if let Err(error) = spawn_result {
            self.as_mut().set_busy(false);
            self.as_mut()
                .set_message(qstring(format!("Could not start password worker: {error}")));
        }
    }

    pub fn region_count(&self) -> i32 {
        i32::try_from(self.rust().region_priority.len()).unwrap_or(i32::MAX)
    }

    pub fn region_at(&self, index: i32) -> QString {
        usize::try_from(index)
            .ok()
            .and_then(|index| self.rust().region_priority.get(index))
            .map(|region| qstring(crate::region_priority::display_name(region)))
            .unwrap_or_default()
    }

    pub fn move_region(mut self: Pin<&mut Self>, from: i32, to: i32) {
        let Ok(from) = usize::try_from(from) else {
            return;
        };
        let Ok(to) = usize::try_from(to) else {
            return;
        };
        let count = self.as_ref().rust().region_priority.len();
        if from >= count || to >= count || from == to {
            return;
        }
        let region = self.as_mut().rust_mut().region_priority.remove(from);
        self.as_mut().rust_mut().region_priority.insert(to, region);
        self.as_mut().sync_primary_region();
        self.as_mut().bump_region_revision();
        self.as_mut().set_message(qstring(
            "Region priority changed. Save settings to apply it to Minerva results.",
        ));
    }

    pub fn reset_region_priority(mut self: Pin<&mut Self>) {
        self.as_mut().rust_mut().region_priority =
            crate::region_priority::default_region_priority();
        self.as_mut().sync_primary_region();
        self.as_mut().bump_region_revision();
        self.as_mut().set_message(qstring(
            "Default region priority restored. Save settings to apply it.",
        ));
    }

    pub fn media_provider_count(&self) -> i32 {
        i32::try_from(self.rust().media_provider_priority.len()).unwrap_or(i32::MAX)
    }

    pub fn media_provider_name_at(&self, index: i32) -> QString {
        usize::try_from(index)
            .ok()
            .and_then(|index| self.rust().media_provider_priority.get(index))
            .map(|provider| qstring(crate::media::provider_display_name(provider)))
            .unwrap_or_default()
    }

    pub fn media_provider_description_at(&self, index: i32) -> QString {
        usize::try_from(index)
            .ok()
            .and_then(|index| self.rust().media_provider_priority.get(index))
            .map(|provider| qstring(crate::media::provider_description(provider)))
            .unwrap_or_default()
    }

    pub fn move_media_provider(mut self: Pin<&mut Self>, from: i32, to: i32) {
        let Ok(from) = usize::try_from(from) else {
            return;
        };
        let Ok(to) = usize::try_from(to) else {
            return;
        };
        let count = self.as_ref().rust().media_provider_priority.len();
        if from >= count || to >= count || from == to {
            return;
        }
        let provider = self
            .as_mut()
            .rust_mut()
            .media_provider_priority
            .remove(from);
        self.as_mut()
            .rust_mut()
            .media_provider_priority
            .insert(to, provider);
        self.as_mut().bump_media_provider_revision();
        self.as_mut().set_message(qstring(
            "Media source priority changed. Save settings to reindex cached media.",
        ));
    }

    pub fn reset_media_provider_priority(mut self: Pin<&mut Self>) {
        self.as_mut().rust_mut().media_provider_priority =
            crate::media::default_provider_priority();
        self.as_mut().bump_media_provider_revision();
        self.as_mut().set_message(qstring(
            "Default media source priority restored. Save settings to reindex cached media.",
        ));
    }

    pub fn refresh_retroarch_shaders(mut self: Pin<&mut Self>) {
        if *self.as_ref().shader_busy() {
            return;
        }
        self.as_mut().rust_mut().shader_generation =
            self.as_ref().rust().shader_generation.wrapping_add(1);
        let generation = self.as_ref().rust().shader_generation;
        let target_override = shader_probe_target();
        self.as_mut().set_shader_busy(true);
        self.as_mut().set_shader_progress(0);
        self.as_mut()
            .set_shader_message(qstring("Inspecting RetroArch shader folders…"));
        let qt_thread = self.as_ref().qt_thread();
        let spawn = std::thread::Builder::new()
            .name("lunchbox-shader-inventory".into())
            .spawn(move || {
                let result = retroarch_shaders::inventory(target_override)
                    .map_err(|error| error.to_string());
                let _ = qt_thread.queue(move |mut model| {
                    model.as_mut().finish_shader_refresh(generation, result);
                });
            });
        if let Err(error) = spawn {
            self.as_mut().set_shader_busy(false);
            self.as_mut().set_shader_message(qstring(format!(
                "Could not start the RetroArch shader scan: {error}"
            )));
        }
    }

    fn finish_shader_refresh(
        mut self: Pin<&mut Self>,
        generation: u64,
        result: Result<ShaderInventory, String>,
    ) {
        if generation != self.as_ref().rust().shader_generation {
            return;
        }
        self.as_mut().set_shader_busy(false);
        match result {
            Ok(inventory) => {
                self.as_mut().apply_shader_inventory(inventory);
                let target_count = self.as_ref().rust().shader_inventory.targets.len();
                let managed_count = self
                    .as_ref()
                    .rust()
                    .shader_inventory
                    .installed_target_count();
                let message = if target_count == 0 {
                    "No RetroArch shader directory could be resolved on this computer.".to_owned()
                } else if managed_count == target_count {
                    format!(
                        "Slang and GLSL packs are managed in {managed_count} RetroArch target{}.",
                        if managed_count == 1 { "" } else { "s" }
                    )
                } else if *self.as_ref().shader_requires_confirmation() {
                    "Existing shader folders were found. Review and confirm before Lunchbox replaces only the official Slang and GLSL pack folders.".to_owned()
                } else {
                    "The official Slang and GLSL packs are ready to install.".to_owned()
                };
                self.as_mut().set_shader_message(qstring(message));
            }
            Err(error) => self.as_mut().set_shader_message(qstring(format!(
                "Could not inspect RetroArch shaders: {error}"
            ))),
        }
        self.as_mut().bump_shader_revision();
    }

    pub fn install_retroarch_shaders(mut self: Pin<&mut Self>, replace_unmanaged: bool) {
        if *self.as_ref().shader_busy() {
            return;
        }
        self.as_mut().rust_mut().shader_generation =
            self.as_ref().rust().shader_generation.wrapping_add(1);
        let generation = self.as_ref().rust().shader_generation;
        let target_override = shader_probe_target();
        let archive_directory = shader_probe_archive_directory();
        let cancel = Arc::new(AtomicBool::new(false));
        self.as_mut().rust_mut().shader_cancel = Some(Arc::clone(&cancel));
        self.as_mut().set_shader_busy(true);
        self.as_mut().set_shader_progress(0);
        self.as_mut()
            .set_shader_message(qstring("Preparing verified RetroArch shader archives…"));
        let qt_thread = self.as_ref().qt_thread();
        let progress_thread = qt_thread.clone();
        let progress: Arc<dyn Fn(ShaderProgress) + Send + Sync> = Arc::new(move |progress| {
            let _ = progress_thread.queue(move |mut model| {
                model.as_mut().update_shader_progress(generation, progress);
            });
        });
        let spawn = std::thread::Builder::new()
            .name("lunchbox-shader-install".into())
            .spawn(move || {
                let result = retroarch_shaders::install(
                    target_override.clone(),
                    archive_directory,
                    replace_unmanaged,
                    &cancel,
                    progress,
                )
                .and_then(|summary| {
                    let inventory = retroarch_shaders::inventory(target_override)?;
                    Ok((summary, inventory))
                })
                .map_err(|error| error.to_string());
                let _ = qt_thread.queue(move |mut model| {
                    model.as_mut().finish_shader_install(generation, result);
                });
            });
        if let Err(error) = spawn {
            self.as_mut().rust_mut().shader_cancel = None;
            self.as_mut().set_shader_busy(false);
            self.as_mut().set_shader_message(qstring(format!(
                "Could not start the RetroArch shader installer: {error}"
            )));
        }
    }

    pub fn cancel_retroarch_shaders(mut self: Pin<&mut Self>) {
        if let Some(cancel) = self.as_ref().rust().shader_cancel.as_ref() {
            cancel.store(true, Ordering::Relaxed);
            self.as_mut().set_shader_message(qstring(
                "Cancelling safely; installed packs remain unchanged until publication…",
            ));
        }
    }

    fn update_shader_progress(mut self: Pin<&mut Self>, generation: u64, progress: ShaderProgress) {
        if generation != self.as_ref().rust().shader_generation || !*self.as_ref().shader_busy() {
            return;
        }
        self.as_mut()
            .set_shader_progress(progress.percent.clamp(0, 100));
        self.as_mut().set_shader_message(qstring(progress.message));
    }

    fn finish_shader_install(
        mut self: Pin<&mut Self>,
        generation: u64,
        result: Result<(ShaderInstallSummary, ShaderInventory), String>,
    ) {
        if generation != self.as_ref().rust().shader_generation {
            return;
        }
        self.as_mut().rust_mut().shader_cancel = None;
        self.as_mut().set_shader_busy(false);
        match result {
            Ok((summary, inventory)) => {
                self.as_mut().apply_shader_inventory(inventory);
                self.as_mut().set_shader_progress(100);
                self.as_mut().set_shader_message(qstring(format!(
                    "Installed {} verified shader files ({}) in {} RetroArch target{}{}.",
                    summary.file_count,
                    format_shader_bytes(summary.unpacked_bytes),
                    summary.target_count,
                    if summary.target_count == 1 { "" } else { "s" },
                    if summary.reused_archives == 0 {
                        String::new()
                    } else {
                        format!(" · {} verified archives reused", summary.reused_archives)
                    }
                )));
            }
            Err(error) => {
                self.as_mut().set_shader_progress(0);
                self.as_mut().set_shader_message(qstring(format!(
                    "RetroArch shader installation failed: {error}"
                )));
            }
        }
        self.as_mut().bump_shader_revision();
    }

    fn apply_shader_inventory(mut self: Pin<&mut Self>, inventory: ShaderInventory) {
        self.as_mut()
            .set_shader_requires_confirmation(inventory.requires_confirmation());
        self.as_mut().rust_mut().shader_inventory = inventory;
    }

    fn bump_shader_revision(mut self: Pin<&mut Self>) {
        let revision = self.as_ref().shader_revision().wrapping_add(1);
        self.as_mut().set_shader_revision(revision);
    }

    pub fn shader_target_count(&self) -> i32 {
        i32::try_from(self.rust().shader_inventory.targets.len()).unwrap_or(i32::MAX)
    }

    pub fn shader_target_path_at(&self, index: i32) -> QString {
        usize::try_from(index)
            .ok()
            .and_then(|index| self.rust().shader_inventory.targets.get(index))
            .map(|target| qstring(target.path.to_string_lossy()))
            .unwrap_or_default()
    }

    pub fn shader_target_detail_at(&self, index: i32) -> QString {
        usize::try_from(index)
            .ok()
            .and_then(|index| self.rust().shader_inventory.targets.get(index))
            .map(|target| qstring(target.detail()))
            .unwrap_or_default()
    }

    pub fn open_shader_target(mut self: Pin<&mut Self>, index: i32) {
        let Ok(index) = usize::try_from(index) else {
            self.as_mut()
                .set_shader_message(qstring("Choose a valid RetroArch shader target."));
            return;
        };
        let path = {
            let this = self.as_ref();
            this.rust()
                .shader_inventory
                .targets
                .get(index)
                .map(|target| target.path.clone())
        };
        let Some(path) = path else {
            self.as_mut()
                .set_shader_message(qstring("Choose a valid RetroArch shader target."));
            return;
        };
        match retroarch_shaders::open_target(&path) {
            Ok(()) => self
                .as_mut()
                .set_shader_message(qstring(format!("Opened {}.", path.display()))),
            Err(error) => self.as_mut().set_shader_message(qstring(format!(
                "Could not open the RetroArch shader folder: {error}"
            ))),
        }
    }

    pub fn refresh_controllers(mut self: Pin<&mut Self>) {
        if *self.as_ref().controller_busy() {
            return;
        }
        self.as_mut().set_controller_busy(true);
        self.as_mut().set_controller_status(qstring(
            "Checking connected controllers and remapping support…",
        ));
        let qt_thread = self.as_ref().qt_thread();
        let spawn_result = std::thread::Builder::new()
            .name("lunchbox-controller-inventory".into())
            .spawn(move || {
                let inventory = controllers::controller_inventory();
                let _ = qt_thread.queue(move |mut model| {
                    model.as_mut().finish_controller_refresh(inventory);
                });
            });
        if let Err(error) = spawn_result {
            self.as_mut().set_controller_busy(false);
            self.as_mut().set_controller_status(qstring(format!(
                "Could not start controller scan: {error}"
            )));
        }
    }

    fn finish_controller_refresh(mut self: Pin<&mut Self>, inventory: ControllerInventory) {
        let status = controller_inventory_status(&inventory);
        let remapping_available = inventory.provider.provider == "inputplumber"
            && inventory.provider.available
            && inventory.provider.service_accessible
            && inventory.managed_device_count > 0;
        if std::env::args().any(|argument| argument == "--controller-ui-probe") {
            println!(
                "LUNCHBOX_CONTROLLER_UI_READY controllers={} managed={} targets={} status={status:?}",
                inventory.controllers.len(),
                inventory.managed_device_count,
                inventory.supported_targets.len()
            );
        }
        self.as_mut().rust_mut().controller_inventory = Some(inventory);
        self.as_mut()
            .set_controller_remapping_available(remapping_available);
        self.as_mut().set_controller_busy(false);
        self.as_mut().set_controller_status(qstring(status));
        self.as_mut().bump_controller_revision();
    }

    pub fn set_controller_mapping_enabled(mut self: Pin<&mut Self>, enabled: bool) {
        self.as_mut().set_controller_enabled(enabled);
        self.as_mut().rust_mut().controller_mapping.enabled = enabled;
        self.as_mut().controller_settings_changed();
    }

    pub fn set_controller_automatic_enabled(mut self: Pin<&mut Self>, enabled: bool) {
        self.as_mut().rust_mut().controller_mapping.automatic = enabled;
        self.as_mut().set_controller_automatic(enabled);
        if enabled {
            self.as_mut().rust_mut().controller_mapping.enabled = true;
            self.as_mut().set_controller_enabled(true);
        }
        self.as_mut().controller_settings_changed();
    }

    pub fn set_controller_calibrated_launch_enabled(mut self: Pin<&mut Self>, enabled: bool) {
        self.as_mut()
            .rust_mut()
            .controller_mapping
            .calibrated_launch = enabled;
        self.as_mut().set_controller_calibrated_launch(enabled);
        self.as_mut().controller_settings_changed();
    }

    pub fn configure_controller_routing(mut self: Pin<&mut Self>, enabled: bool) {
        if *self.as_ref().controller_busy() {
            return;
        }
        self.as_mut().set_controller_busy(true);
        let qt_thread = self.as_ref().qt_thread();
        let result = std::thread::Builder::new()
            .name("lunchbox-controller-routing".into())
            .spawn(move || {
                let result = controllers::configure_linux_routing(enabled);
                let _ = qt_thread.queue(move |mut model| {
                    model.as_mut().set_controller_busy(false);
                    match result {
                        Ok(()) => model.as_mut().refresh_controllers(),
                        Err(error) => model.as_mut().set_controller_status(qstring(format!(
                            "Controller routing could not be changed: {error}"
                        ))),
                    }
                });
            });
        if let Err(error) = result {
            self.as_mut().set_controller_busy(false);
            self.as_mut()
                .set_controller_status(qstring(error.to_string()));
        }
    }

    pub fn controller_layout_at(&self, index: i32) -> QString {
        self.controller_at(index)
            .map(|device| {
                qstring(controllers::device_layout(
                    &self.rust().controller_mapping,
                    &device,
                ))
            })
            .unwrap_or_else(|| qstring("auto"))
    }

    pub fn choose_controller_layout(mut self: Pin<&mut Self>, index: i32, layout: QString) {
        let layout = layout.to_string();
        if !controllers::DEVICE_LAYOUTS
            .iter()
            .any(|(id, _)| *id == layout)
        {
            return;
        }
        let Some(device) = self.as_ref().controller_at(index) else {
            return;
        };
        self.as_mut()
            .rust_mut()
            .controller_mapping
            .device_layouts
            .insert(device.stable_id, layout);
        self.as_mut().controller_settings_changed();
    }

    pub fn controller_preference_at(&self, system: QString) -> i32 {
        let Some(id) = self
            .rust()
            .controller_mapping
            .preferred_devices
            .get(&system.to_string())
        else {
            return 0;
        };
        (0..self.controller_count())
            .find(|index| {
                self.controller_at(*index)
                    .is_some_and(|device| device.stable_id == *id)
            })
            .map_or(0, |index| index + 1)
    }

    pub fn choose_preferred_controller(mut self: Pin<&mut Self>, system: QString, index: i32) {
        let system = system.to_string();
        if !controllers::SYSTEM_LAYOUTS
            .iter()
            .any(|(id, _)| *id == system)
        {
            return;
        }
        if index == 0 {
            self.as_mut()
                .rust_mut()
                .controller_mapping
                .preferred_devices
                .remove(&system);
        } else if let Some(device) = self.as_ref().controller_at(index - 1) {
            self.as_mut()
                .rust_mut()
                .controller_mapping
                .preferred_devices
                .insert(system, device.stable_id);
        }
        self.as_mut().controller_settings_changed();
    }

    pub fn preferred_controller_profile(&self, system: QString) -> QString {
        let mapping = &self.rust().controller_mapping;
        qstring(
            mapping
                .preferred_devices
                .get(&system.to_string())
                .and_then(|device| mapping.device_system_profiles.get(device))
                .and_then(|profiles| profiles.get(&system.to_string()))
                .map(String::as_str)
                .unwrap_or(""),
        )
    }

    pub fn choose_preferred_controller_profile(
        mut self: Pin<&mut Self>,
        system: QString,
        profile: QString,
    ) {
        let system = system.to_string();
        let profile = profile.to_string();
        let this = self.as_ref();
        let mapping = &this.rust().controller_mapping;
        if !profile.is_empty()
            && profile != "none"
            && profile != controllers::TWO_BUTTON_CLOCKWISE_PROFILE_ID
            && !mapping
                .custom_profiles
                .iter()
                .any(|custom| custom.id == profile)
        {
            return;
        }
        let Some(device) = mapping.preferred_devices.get(&system).cloned() else {
            return;
        };
        let this = self.as_mut();
        let mut data = this.rust_mut();
        let profiles = data
            .controller_mapping
            .device_system_profiles
            .entry(device)
            .or_default();
        if profile.is_empty() {
            profiles.remove(&system);
        } else {
            profiles.insert(system, profile);
        }
        drop(data);
        self.as_mut().controller_settings_changed();
    }

    pub fn choose_default_controller_target(mut self: Pin<&mut Self>, target: QString) {
        let target = target.to_string();
        let target = target.trim();
        let supported = self
            .as_ref()
            .rust()
            .controller_inventory
            .as_ref()
            .is_some_and(|inventory| {
                inventory
                    .supported_targets
                    .iter()
                    .any(|candidate| candidate.id == target)
            });
        if !supported {
            return;
        }
        self.as_mut().set_controller_output_target(qstring(target));
        self.as_mut().rust_mut().controller_mapping.output_target = target.to_owned();
        self.as_mut().controller_settings_changed();
    }

    pub fn controller_count(&self) -> i32 {
        self.rust()
            .controller_inventory
            .as_ref()
            .map(|inventory| {
                controllers::ordered_controllers(inventory, &self.rust().controller_mapping).len()
            })
            .and_then(|count| i32::try_from(count).ok())
            .unwrap_or(0)
    }

    pub fn controller_name_at(&self, index: i32) -> QString {
        self.controller_at(index)
            .map(|controller| {
                if let Some(name) = self
                    .rust()
                    .controller_mapping
                    .device_names
                    .get(&controller.stable_id)
                {
                    return qstring(name);
                }
                if controller.vendor_id.as_deref() == Some("28de")
                    && controller.product_id.as_deref() == Some("11ff")
                {
                    return qstring("Steam Input virtual controller");
                }
                if controller.vendor_id.as_deref() == Some("045e")
                    && controller.product_id.as_deref() == Some("028e")
                {
                    return qstring(format!(
                        "USB XInput controller · revision {} · {}",
                        controller.version.as_deref().unwrap_or("unknown"),
                        controller.device_path.display()
                    ));
                }
                qstring(controller.name)
            })
            .unwrap_or_default()
    }

    pub fn controller_alias_at(&self, index: i32) -> QString {
        self.controller_at(index)
            .and_then(|device| {
                self.rust()
                    .controller_mapping
                    .device_names
                    .get(&device.stable_id)
                    .cloned()
            })
            .map(qstring)
            .unwrap_or_default()
    }

    pub fn controller_key_at(&self, index: i32) -> QString {
        self.controller_at(index)
            .map(|device| qstring(device.stable_id))
            .unwrap_or_default()
    }

    pub fn controller_key_for_input(&self, key: QString) -> QString {
        let Some(inventory) = &self.rust().controller_inventory else {
            return QString::default();
        };
        inventory
            .controllers
            .iter()
            .find(|device| {
                controllers::controller_receives_input(
                    &inventory.controllers,
                    &device.stable_id,
                    &key.to_string(),
                )
            })
            .map(|device| qstring(&device.stable_id))
            .unwrap_or_default()
    }

    pub fn controller_catalog_json(&self) -> QString {
        let catalog = crate::controller_catalog::catalog();
        qstring(serde_json::json!({"layouts":catalog.layouts,"emulator_profiles":catalog.emulator_profiles,"host_os":std::env::consts::OS}).to_string())
    }

    pub fn use_sdl3_controller_mapping(mut self: Pin<&mut Self>, device: QString) -> QString {
        let id = device.to_string();
        let result = (|| -> anyhow::Result<_> {
            anyhow::ensure!(!*self.as_ref().busy(), "Wait for settings to finish saving");
            let calibration = crate::controller_sdl3::standard_calibration(&id)?;
            SettingsStore::open_default()?.save_controller_calibration(&id, &calibration)?;
            Ok(calibration)
        })();
        match result {
            Ok(calibration) => {
                self.as_mut()
                    .rust_mut()
                    .controller_mapping
                    .calibrations
                    .insert(id, calibration);
                self.as_mut().bump_controller_revision();
                qstring("")
            }
            Err(error) => qstring(format!("{error:#}")),
        }
    }

    pub fn controller_coverage_json(&self) -> QString {
        let report = crate::controller_coverage::report(
            &self.rust().controller_mapping.launch_mode_selections,
        )
        .unwrap_or_else(|error| serde_json::json!({"error":format!("{error:#}")}));
        qstring(report.to_string())
    }

    pub fn emulator_platform_locations_json(&self, emulator: QString) -> QString {
        let bases = crate::platform_locations::LocationBases::detect();
        let result = crate::platform_locations::load_records().and_then(|records| {
            crate::platform_locations::locations_for_emulator_name(
                &records,
                &emulator.to_string(),
                &bases,
            )
        });
        match result {
            Ok(text) => qstring(text),
            Err(error) => qstring(serde_json::json!({"error": format!("{error:#}")}).to_string()),
        }
    }

    pub fn fbneo_controller_setups_json(&self) -> QString {
        let rows = self.rust().controller_mapping.fbneo_launches.iter().map(|setup|
            serde_json::json!({
                "key": setup.identity_key(), "emulator_id": setup.emulator_id,
                "core": setup.core, "content": setup.content,
                "players": setup.players.len(),
                "assignments": setup.players.iter().map(|player| player.assignments.len()).sum::<usize>(),
            })).collect::<Vec<_>>();
        qstring(serde_json::to_string(&rows).expect("FBNeo setup summaries serialize"))
    }

    pub fn mame_controller_setups_json(&self) -> QString {
        let rows: Vec<_> = self
            .rust()
            .controller_mapping
            .mame_launches
            .iter()
            .map(|setup| {
                serde_json::json!({"key":setup.identity_key(), "machine":setup.machine,
                "content":setup.content, "players":setup.players.len(),
                "snapshot_schema":setup.reviewed_snapshot.schema_version,
                "needs_reinspection":setup.reviewed_snapshot.needs_reinspection()})
            })
            .collect();
        qstring(serde_json::to_string(&rows).expect("MAME setup summaries serialize"))
    }

    pub fn mame_arcade_layout(&self) -> QString {
        use crate::controller_mame::DigitalLayout;
        qstring(match self.rust().controller_mapping.mame_arcade_layout {
            None => "disabled",
            Some(DigitalLayout::Automatic) => "automatic",
            Some(DigitalLayout::SixButton) => "six_button",
            Some(DigitalLayout::EightButton) => "eight_button",
            Some(DigitalLayout::NeoGeo) => "neo_geo",
            Some(DigitalLayout::FixedChannels) => "fixed_channels",
        })
    }

    pub fn duckstation_setups_json(&self) -> QString {
        qstring(
            serde_json::to_string_pretty(&self.rust().controller_mapping.duckstation_launches)
                .expect("DuckStation setups serialize"),
        )
    }

    pub fn review_duckstation_setups(&self, configuration: QString) -> QString {
        let result = (|| -> anyhow::Result<Vec<serde_json::Value>> {
            let setups: Vec<crate::controller_duckstation::SavedSetup> =
                serde_json::from_str(&configuration.to_string())?;
            crate::controller_duckstation::validate_setups(&setups)?;
            setups
                .iter()
                .map(|setup| setup.review(&self.rust().controller_mapping.calibrations))
                .collect()
        })();
        qstring(match result {
            Ok(reviews) => serde_json::json!({"setups":reviews}).to_string(),
            Err(error) => serde_json::json!({"error":format!("{error:#}")}).to_string(),
        })
    }

    pub fn stage_duckstation_setups(mut self: Pin<&mut Self>, configuration: QString) -> QString {
        let result = (|| -> anyhow::Result<Vec<crate::controller_duckstation::SavedSetup>> {
            let setups: Vec<crate::controller_duckstation::SavedSetup> =
                serde_json::from_str(&configuration.to_string())?;
            crate::controller_duckstation::validate_setups(&setups)?;
            for setup in &setups {
                setup.review(&self.rust().controller_mapping.calibrations)?;
            }
            Ok(setups)
        })();
        match result {
            Ok(setups) => {
                self.as_mut()
                    .rust_mut()
                    .controller_mapping
                    .duckstation_launches = setups;
                self.as_mut().controller_settings_changed();
                qstring("")
            }
            Err(error) => qstring(format!("{error:#}")),
        }
    }

    pub fn relative_device_settings_json(&self) -> QString {
        qstring(
            serde_json::to_string_pretty(&self.rust().controller_mapping.relative_devices)
                .expect("Relative device settings serialize"),
        )
    }

    pub fn dolphin_setups_json(&self) -> QString {
        qstring(
            serde_json::to_string_pretty(&self.rust().controller_mapping.dolphin_launches)
                .expect("Dolphin setups serialize"),
        )
    }

    pub fn review_dolphin_setups(&self, configuration: QString) -> QString {
        let result = (|| -> anyhow::Result<Vec<serde_json::Value>> {
            let text = configuration.to_string();
            anyhow::ensure!(
                text.len() <= 2 * 1024 * 1024,
                "Dolphin setup text exceeds size limit"
            );
            let setups: Vec<crate::controller_dolphin::standalone::settings::SavedSetup> =
                serde_json::from_str(&text)?;
            crate::controller_dolphin::standalone::settings::validate_setups(&setups)?;
            setups
                .iter()
                .map(|setup| setup.review(&self.rust().controller_mapping.calibrations))
                .collect()
        })();
        qstring(match result {
            Ok(reviews) => serde_json::json!({"setups":reviews}).to_string(),
            Err(error) => serde_json::json!({"error":format!("{error:#}")}).to_string(),
        })
    }

    pub fn stage_dolphin_setups(mut self: Pin<&mut Self>, configuration: QString) -> QString {
        let result = (|| -> anyhow::Result<Vec<crate::controller_dolphin::standalone::settings::SavedSetup>> {
            let text = configuration.to_string();
            anyhow::ensure!(
                text.len() <= 2 * 1024 * 1024,
                "Dolphin setup text exceeds size limit"
            );
            let setups: Vec<crate::controller_dolphin::standalone::settings::SavedSetup> =
                serde_json::from_str(&text)?;
            crate::controller_dolphin::standalone::settings::validate_setups(&setups)?;
            for setup in &setups {
                setup.review(&self.rust().controller_mapping.calibrations)?;
            }
            Ok(setups)
        })();
        match result {
            Ok(setups) => {
                self.as_mut().rust_mut().controller_mapping.dolphin_launches = setups;
                self.as_mut().controller_settings_changed();
                qstring("")
            }
            Err(error) => qstring(format!("{error:#}")),
        }
    }

    pub fn fceux_setups_json(&self) -> QString {
        qstring(
            serde_json::to_string_pretty(&self.rust().controller_mapping.fceux_launches)
                .expect("FCEUX Qt setups serialize"),
        )
    }

    pub fn review_fceux_setups(&self, configuration: QString) -> QString {
        let result = (|| -> anyhow::Result<Vec<serde_json::Value>> {
            let text = configuration.to_string();
            anyhow::ensure!(
                text.len() <= 2 * 1024 * 1024,
                "FCEUX Qt setup text exceeds size limit"
            );
            let setups: Vec<crate::controller_fceux::settings::SavedSetup> =
                serde_json::from_str(&text)?;
            crate::controller_fceux::settings::validate_setups(&setups)?;
            setups
                .iter()
                .map(|setup| setup.review(&self.rust().controller_mapping.calibrations))
                .collect()
        })();
        qstring(match result {
            Ok(reviews) => serde_json::json!({"setups":reviews}).to_string(),
            Err(error) => serde_json::json!({"error":format!("{error:#}")}).to_string(),
        })
    }

    pub fn stage_fceux_setups(mut self: Pin<&mut Self>, configuration: QString) -> QString {
        let result = (|| -> anyhow::Result<Vec<crate::controller_fceux::settings::SavedSetup>> {
            let text = configuration.to_string();
            anyhow::ensure!(
                text.len() <= 2 * 1024 * 1024,
                "FCEUX Qt setup text exceeds size limit"
            );
            let setups: Vec<crate::controller_fceux::settings::SavedSetup> =
                serde_json::from_str(&text)?;
            crate::controller_fceux::settings::validate_setups(&setups)?;
            for setup in &setups {
                setup.review(&self.rust().controller_mapping.calibrations)?;
            }
            Ok(setups)
        })();
        match result {
            Ok(setups) => {
                self.as_mut().rust_mut().controller_mapping.fceux_launches = setups;
                self.as_mut().controller_settings_changed();
                qstring("")
            }
            Err(error) => qstring(format!("{error:#}")),
        }
    }

    pub fn sameboy_setups_json(&self) -> QString {
        qstring(
            serde_json::to_string_pretty(&self.rust().controller_mapping.sameboy_launches)
                .expect("SameBoy SDL setups serialize"),
        )
    }

    pub fn review_sameboy_setups(&self, configuration: QString) -> QString {
        let result = (|| -> anyhow::Result<Vec<serde_json::Value>> {
            let text = configuration.to_string();
            anyhow::ensure!(
                text.len() <= 2 * 1024 * 1024,
                "SameBoy SDL setup text exceeds size limit"
            );
            let setups: Vec<crate::controller_sameboy::settings::SavedSetup> =
                serde_json::from_str(&text)?;
            crate::controller_sameboy::settings::validate_setups(&setups)?;
            setups
                .iter()
                .map(|setup| setup.review(&self.rust().controller_mapping.calibrations))
                .collect()
        })();
        qstring(match result {
            Ok(reviews) => serde_json::json!({"setups":reviews}).to_string(),
            Err(error) => serde_json::json!({"error":format!("{error:#}")}).to_string(),
        })
    }

    pub fn stage_sameboy_setups(mut self: Pin<&mut Self>, configuration: QString) -> QString {
        let result =
            (|| -> anyhow::Result<Vec<crate::controller_sameboy::settings::SavedSetup>> {
                let text = configuration.to_string();
                anyhow::ensure!(
                    text.len() <= 2 * 1024 * 1024,
                    "SameBoy SDL setup text exceeds size limit"
                );
                let setups: Vec<crate::controller_sameboy::settings::SavedSetup> =
                    serde_json::from_str(&text)?;
                crate::controller_sameboy::settings::validate_setups(&setups)?;
                for setup in &setups {
                    setup.review(&self.rust().controller_mapping.calibrations)?;
                }
                Ok(setups)
            })();
        match result {
            Ok(setups) => {
                self.as_mut().rust_mut().controller_mapping.sameboy_launches = setups;
                self.as_mut().controller_settings_changed();
                qstring("")
            }
            Err(error) => qstring(format!("{error:#}")),
        }
    }

    pub fn desmume_native_setups_json(&self) -> QString {
        qstring(
            serde_json::to_string_pretty(&self.rust().controller_mapping.desmume_native_launches)
                .expect("desmume_native setups serialize"),
        )
    }

    pub fn review_desmume_native_setups(&self, configuration: QString) -> QString {
        let result = (|| -> anyhow::Result<Vec<serde_json::Value>> {
            let text = configuration.to_string();
            anyhow::ensure!(
                text.len() <= 2 * 1024 * 1024,
                "desmume_native setup text exceeds size limit"
            );
            let setups: Vec<crate::controller_desmume_native::settings::SavedSetup> =
                serde_json::from_str(&text)?;
            crate::controller_desmume_native::settings::validate_setups(&setups)?;
            setups
                .iter()
                .map(|setup| setup.review(&self.rust().controller_mapping.calibrations))
                .collect()
        })();
        qstring(match result {
            Ok(reviews) => serde_json::json!({"setups":reviews}).to_string(),
            Err(error) => serde_json::json!({"error":format!("{error:#}")}).to_string(),
        })
    }

    pub fn stage_desmume_native_setups(
        mut self: Pin<&mut Self>,
        configuration: QString,
    ) -> QString {
        let result =
            (|| -> anyhow::Result<Vec<crate::controller_desmume_native::settings::SavedSetup>> {
                let text = configuration.to_string();
                anyhow::ensure!(
                    text.len() <= 2 * 1024 * 1024,
                    "desmume_native setup text exceeds size limit"
                );
                let setups: Vec<crate::controller_desmume_native::settings::SavedSetup> =
                    serde_json::from_str(&text)?;
                crate::controller_desmume_native::settings::validate_setups(&setups)?;
                for setup in &setups {
                    setup.review(&self.rust().controller_mapping.calibrations)?;
                }
                Ok(setups)
            })();
        match result {
            Ok(setups) => {
                self.as_mut()
                    .rust_mut()
                    .controller_mapping
                    .desmume_native_launches = setups;
                self.as_mut().controller_settings_changed();
                qstring("")
            }
            Err(error) => qstring(format!("{error:#}")),
        }
    }

    pub fn openmsx_native_setups_json(&self) -> QString {
        qstring(
            serde_json::to_string_pretty(&self.rust().controller_mapping.openmsx_native_launches)
                .expect("openmsx_native setups serialize"),
        )
    }

    pub fn review_openmsx_native_setups(&self, configuration: QString) -> QString {
        let result = (|| -> anyhow::Result<Vec<serde_json::Value>> {
            let text = configuration.to_string();
            anyhow::ensure!(
                text.len() <= 2 * 1024 * 1024,
                "openmsx_native setup text exceeds size limit"
            );
            let setups: Vec<crate::controller_openmsx_native::settings::SavedSetup> =
                serde_json::from_str(&text)?;
            crate::controller_openmsx_native::settings::validate_setups(&setups)?;
            setups
                .iter()
                .map(|setup| setup.review(&self.rust().controller_mapping.calibrations))
                .collect()
        })();
        qstring(match result {
            Ok(reviews) => serde_json::json!({"setups":reviews}).to_string(),
            Err(error) => serde_json::json!({"error":format!("{error:#}")}).to_string(),
        })
    }

    pub fn stage_openmsx_native_setups(
        mut self: Pin<&mut Self>,
        configuration: QString,
    ) -> QString {
        let result =
            (|| -> anyhow::Result<Vec<crate::controller_openmsx_native::settings::SavedSetup>> {
                let text = configuration.to_string();
                anyhow::ensure!(
                    text.len() <= 2 * 1024 * 1024,
                    "openmsx_native setup text exceeds size limit"
                );
                let setups: Vec<crate::controller_openmsx_native::settings::SavedSetup> =
                    serde_json::from_str(&text)?;
                crate::controller_openmsx_native::settings::validate_setups(&setups)?;
                for setup in &setups {
                    setup.review(&self.rust().controller_mapping.calibrations)?;
                }
                Ok(setups)
            })();
        match result {
            Ok(setups) => {
                self.as_mut()
                    .rust_mut()
                    .controller_mapping
                    .openmsx_native_launches = setups;
                self.as_mut().controller_settings_changed();
                qstring("")
            }
            Err(error) => qstring(format!("{error:#}")),
        }
    }

    pub fn mesen2_native_setups_json(&self) -> QString {
        qstring(
            serde_json::to_string_pretty(&self.rust().controller_mapping.mesen2_native_launches)
                .expect("mesen2_native setups serialize"),
        )
    }

    pub fn review_mesen2_native_setups(&self, configuration: QString) -> QString {
        let result = (|| -> anyhow::Result<Vec<serde_json::Value>> {
            let text = configuration.to_string();
            anyhow::ensure!(
                text.len() <= 2 * 1024 * 1024,
                "mesen2_native setup text exceeds size limit"
            );
            let setups: Vec<crate::controller_mesen2_native::settings::SavedSetup> =
                serde_json::from_str(&text)?;
            crate::controller_mesen2_native::settings::validate_setups(&setups)?;
            setups
                .iter()
                .map(|setup| setup.review(&self.rust().controller_mapping.calibrations))
                .collect()
        })();
        qstring(match result {
            Ok(reviews) => serde_json::json!({"setups":reviews}).to_string(),
            Err(error) => serde_json::json!({"error":format!("{error:#}")}).to_string(),
        })
    }

    pub fn stage_mesen2_native_setups(mut self: Pin<&mut Self>, configuration: QString) -> QString {
        let result =
            (|| -> anyhow::Result<Vec<crate::controller_mesen2_native::settings::SavedSetup>> {
                let text = configuration.to_string();
                anyhow::ensure!(
                    text.len() <= 2 * 1024 * 1024,
                    "mesen2_native setup text exceeds size limit"
                );
                let setups: Vec<crate::controller_mesen2_native::settings::SavedSetup> =
                    serde_json::from_str(&text)?;
                crate::controller_mesen2_native::settings::validate_setups(&setups)?;
                for setup in &setups {
                    setup.review(&self.rust().controller_mapping.calibrations)?;
                }
                Ok(setups)
            })();
        match result {
            Ok(setups) => {
                self.as_mut()
                    .rust_mut()
                    .controller_mapping
                    .mesen2_native_launches = setups;
                self.as_mut().controller_settings_changed();
                qstring("")
            }
            Err(error) => qstring(format!("{error:#}")),
        }
    }

    pub fn blastem_native_setups_json(&self) -> QString {
        qstring(
            serde_json::to_string_pretty(&self.rust().controller_mapping.blastem_native_launches)
                .expect("blastem_native setups serialize"),
        )
    }

    pub fn review_blastem_native_setups(&self, configuration: QString) -> QString {
        let result = (|| -> anyhow::Result<Vec<serde_json::Value>> {
            let text = configuration.to_string();
            anyhow::ensure!(
                text.len() <= 2 * 1024 * 1024,
                "blastem_native setup text exceeds size limit"
            );
            let setups: Vec<crate::controller_blastem_native::settings::SavedSetup> =
                serde_json::from_str(&text)?;
            crate::controller_blastem_native::settings::validate_setups(&setups)?;
            setups
                .iter()
                .map(|setup| setup.review(&self.rust().controller_mapping.calibrations))
                .collect()
        })();
        qstring(match result {
            Ok(reviews) => serde_json::json!({"setups":reviews}).to_string(),
            Err(error) => serde_json::json!({"error":format!("{error:#}")}).to_string(),
        })
    }

    pub fn stage_blastem_native_setups(
        mut self: Pin<&mut Self>,
        configuration: QString,
    ) -> QString {
        let result =
            (|| -> anyhow::Result<Vec<crate::controller_blastem_native::settings::SavedSetup>> {
                let text = configuration.to_string();
                anyhow::ensure!(
                    text.len() <= 2 * 1024 * 1024,
                    "blastem_native setup text exceeds size limit"
                );
                let setups: Vec<crate::controller_blastem_native::settings::SavedSetup> =
                    serde_json::from_str(&text)?;
                crate::controller_blastem_native::settings::validate_setups(&setups)?;
                for setup in &setups {
                    setup.review(&self.rust().controller_mapping.calibrations)?;
                }
                Ok(setups)
            })();
        match result {
            Ok(setups) => {
                self.as_mut()
                    .rust_mut()
                    .controller_mapping
                    .blastem_native_launches = setups;
                self.as_mut().controller_settings_changed();
                qstring("")
            }
            Err(error) => qstring(format!("{error:#}")),
        }
    }

    pub fn jgenesis_native_setups_json(&self) -> QString {
        qstring(
            serde_json::to_string_pretty(&self.rust().controller_mapping.jgenesis_native_launches)
                .expect("jgenesis setups serialize"),
        )
    }

    pub fn review_jgenesis_native_setups(&self, configuration: QString) -> QString {
        let result = (|| -> anyhow::Result<Vec<serde_json::Value>> {
            let text = configuration.to_string();
            anyhow::ensure!(
                text.len() <= 2 * 1024 * 1024,
                "jgenesis setup text exceeds size limit"
            );
            let setups: Vec<crate::controller_jgenesis_native::settings::SavedSetup> =
                serde_json::from_str(&text)?;
            crate::controller_jgenesis_native::settings::validate_setups(&setups)?;
            setups
                .iter()
                .map(|setup| setup.review(&self.rust().controller_mapping.calibrations))
                .collect()
        })();
        qstring(match result {
            Ok(reviews) => serde_json::json!({"setups":reviews}).to_string(),
            Err(error) => serde_json::json!({"error":format!("{error:#}")}).to_string(),
        })
    }

    pub fn stage_jgenesis_native_setups(
        mut self: Pin<&mut Self>,
        configuration: QString,
    ) -> QString {
        let result =
            (|| -> anyhow::Result<Vec<crate::controller_jgenesis_native::settings::SavedSetup>> {
                let text = configuration.to_string();
                anyhow::ensure!(
                    text.len() <= 2 * 1024 * 1024,
                    "jgenesis setup text exceeds size limit"
                );
                let setups: Vec<crate::controller_jgenesis_native::settings::SavedSetup> =
                    serde_json::from_str(&text)?;
                crate::controller_jgenesis_native::settings::validate_setups(&setups)?;
                for setup in &setups {
                    setup.review(&self.rust().controller_mapping.calibrations)?;
                }
                Ok(setups)
            })();
        match result {
            Ok(setups) => {
                self.as_mut()
                    .rust_mut()
                    .controller_mapping
                    .jgenesis_native_launches = setups;
                self.as_mut().controller_settings_changed();
                qstring("")
            }
            Err(error) => qstring(format!("{error:#}")),
        }
    }

    pub fn gopher64_native_setups_json(&self) -> QString {
        qstring(
            serde_json::to_string_pretty(&self.rust().controller_mapping.gopher64_native_launches)
                .expect("gopher64 setups serialize"),
        )
    }

    pub fn b2_native_setups_json(&self) -> QString {
        qstring(
            serde_json::to_string_pretty(&self.rust().controller_mapping.b2_native_launches)
                .expect("b2 setups serialize"),
        )
    }

    pub fn hypseus_native_setups_json(&self) -> QString {
        qstring(
            serde_json::to_string_pretty(&self.rust().controller_mapping.hypseus_native_launches)
                .expect("Hypseus setups serialize"),
        )
    }

    pub fn review_hypseus_native_setups(&self, configuration: QString) -> QString {
        let result = (|| -> anyhow::Result<Vec<serde_json::Value>> {
            let text = configuration.to_string();
            anyhow::ensure!(
                text.len() <= 2 * 1024 * 1024,
                "Hypseus setup text exceeds size limit"
            );
            let setups: Vec<crate::controller_hypseus_singe_native::settings::SavedSetup> =
                serde_json::from_str(&text)?;
            crate::controller_hypseus_singe_native::settings::validate_setups(&setups)?;
            setups.iter().map(|setup| setup.review()).collect()
        })();
        qstring(match result {
            Ok(reviews) => serde_json::json!({"setups":reviews}).to_string(),
            Err(error) => serde_json::json!({"error":format!("{error:#}")}).to_string(),
        })
    }

    pub fn stage_hypseus_native_setups(
        mut self: Pin<&mut Self>,
        configuration: QString,
    ) -> QString {
        let result = (|| -> anyhow::Result<
            Vec<crate::controller_hypseus_singe_native::settings::SavedSetup>,
        > {
            let text = configuration.to_string();
            anyhow::ensure!(
                text.len() <= 2 * 1024 * 1024,
                "Hypseus setup text exceeds size limit"
            );
            let setups: Vec<crate::controller_hypseus_singe_native::settings::SavedSetup> =
                serde_json::from_str(&text)?;
            crate::controller_hypseus_singe_native::settings::validate_setups(&setups)?;
            for setup in &setups {
                setup.review()?;
            }
            Ok(setups)
        })();
        match result {
            Ok(setups) => {
                self.as_mut()
                    .rust_mut()
                    .controller_mapping
                    .hypseus_native_launches = setups;
                self.as_mut().controller_settings_changed();
                qstring("")
            }
            Err(error) => qstring(format!("{error:#}")),
        }
    }

    pub fn review_b2_native_setups(&self, configuration: QString) -> QString {
        let result = (|| -> anyhow::Result<Vec<serde_json::Value>> {
            let text = configuration.to_string();
            anyhow::ensure!(
                text.len() <= 2 * 1024 * 1024,
                "b2 setup text exceeds size limit"
            );
            let setups: Vec<crate::controller_b2_native::settings::SavedSetup> =
                serde_json::from_str(&text)?;
            crate::controller_b2_native::settings::validate_setups(&setups)?;
            setups.iter().map(|setup| setup.review()).collect()
        })();
        qstring(match result {
            Ok(reviews) => serde_json::json!({"setups":reviews}).to_string(),
            Err(error) => serde_json::json!({"error":format!("{error:#}")}).to_string(),
        })
    }

    pub fn stage_b2_native_setups(mut self: Pin<&mut Self>, configuration: QString) -> QString {
        let result =
            (|| -> anyhow::Result<Vec<crate::controller_b2_native::settings::SavedSetup>> {
                let text = configuration.to_string();
                anyhow::ensure!(
                    text.len() <= 2 * 1024 * 1024,
                    "b2 setup text exceeds size limit"
                );
                let setups: Vec<crate::controller_b2_native::settings::SavedSetup> =
                    serde_json::from_str(&text)?;
                crate::controller_b2_native::settings::validate_setups(&setups)?;
                for setup in &setups {
                    setup.review()?;
                }
                Ok(setups)
            })();
        match result {
            Ok(setups) => {
                self.as_mut()
                    .rust_mut()
                    .controller_mapping
                    .b2_native_launches = setups;
                self.as_mut().controller_settings_changed();
                qstring("")
            }
            Err(error) => qstring(format!("{error:#}")),
        }
    }

    pub fn review_gopher64_native_setups(&self, configuration: QString) -> QString {
        let result = (|| -> anyhow::Result<Vec<serde_json::Value>> {
            let text = configuration.to_string();
            anyhow::ensure!(
                text.len() <= 2 * 1024 * 1024,
                "gopher64 setup text exceeds size limit"
            );
            let setups: Vec<crate::controller_gopher64_native::settings::SavedSetup> =
                serde_json::from_str(&text)?;
            crate::controller_gopher64_native::settings::validate_setups(&setups)?;
            setups
                .iter()
                .map(|setup| setup.review(&self.rust().controller_mapping.calibrations))
                .collect()
        })();
        qstring(match result {
            Ok(reviews) => serde_json::json!({"setups":reviews}).to_string(),
            Err(error) => serde_json::json!({"error":format!("{error:#}")}).to_string(),
        })
    }

    pub fn stage_gopher64_native_setups(
        mut self: Pin<&mut Self>,
        configuration: QString,
    ) -> QString {
        let result =
            (|| -> anyhow::Result<Vec<crate::controller_gopher64_native::settings::SavedSetup>> {
                let text = configuration.to_string();
                anyhow::ensure!(
                    text.len() <= 2 * 1024 * 1024,
                    "gopher64 setup text exceeds size limit"
                );
                let setups: Vec<crate::controller_gopher64_native::settings::SavedSetup> =
                    serde_json::from_str(&text)?;
                crate::controller_gopher64_native::settings::validate_setups(&setups)?;
                for setup in &setups {
                    setup.review(&self.rust().controller_mapping.calibrations)?;
                }
                Ok(setups)
            })();
        match result {
            Ok(setups) => {
                self.as_mut()
                    .rust_mut()
                    .controller_mapping
                    .gopher64_native_launches = setups;
                self.as_mut().controller_settings_changed();
                qstring("")
            }
            Err(error) => qstring(format!("{error:#}")),
        }
    }

    pub fn rmg_native_setups_json(&self) -> QString {
        qstring(
            serde_json::to_string_pretty(&self.rust().controller_mapping.rmg_native_launches)
                .expect("RMG setups serialize"),
        )
    }

    pub fn review_rmg_native_setups(&self, configuration: QString) -> QString {
        let result = (|| -> anyhow::Result<Vec<serde_json::Value>> {
            let text = configuration.to_string();
            anyhow::ensure!(
                text.len() <= 2 * 1024 * 1024,
                "RMG setup text exceeds size limit"
            );
            let setups: Vec<crate::controller_rmg_native::settings::SavedSetup> =
                serde_json::from_str(&text)?;
            crate::controller_rmg_native::settings::validate_setups(&setups)?;
            setups
                .iter()
                .map(|setup| setup.review(&self.rust().controller_mapping.calibrations))
                .collect()
        })();
        qstring(match result {
            Ok(reviews) => serde_json::json!({"setups":reviews}).to_string(),
            Err(error) => serde_json::json!({"error":format!("{error:#}")}).to_string(),
        })
    }

    pub fn stage_rmg_native_setups(mut self: Pin<&mut Self>, configuration: QString) -> QString {
        let result =
            (|| -> anyhow::Result<Vec<crate::controller_rmg_native::settings::SavedSetup>> {
                let text = configuration.to_string();
                anyhow::ensure!(
                    text.len() <= 2 * 1024 * 1024,
                    "RMG setup text exceeds size limit"
                );
                let setups: Vec<crate::controller_rmg_native::settings::SavedSetup> =
                    serde_json::from_str(&text)?;
                crate::controller_rmg_native::settings::validate_setups(&setups)?;
                for setup in &setups {
                    setup.review(&self.rust().controller_mapping.calibrations)?;
                }
                Ok(setups)
            })();
        match result {
            Ok(setups) => {
                self.as_mut()
                    .rust_mut()
                    .controller_mapping
                    .rmg_native_launches = setups;
                self.as_mut().controller_settings_changed();
                qstring("")
            }
            Err(error) => qstring(format!("{error:#}")),
        }
    }

    pub fn simple64_native_setups_json(&self) -> QString {
        qstring(
            serde_json::to_string_pretty(&self.rust().controller_mapping.simple64_native_launches)
                .expect("simple64 setups serialize"),
        )
    }

    pub fn review_simple64_native_setups(&self, configuration: QString) -> QString {
        let result = (|| -> anyhow::Result<Vec<serde_json::Value>> {
            let text = configuration.to_string();
            anyhow::ensure!(
                text.len() <= 2 * 1024 * 1024,
                "simple64 setup text exceeds size limit"
            );
            let setups: Vec<crate::controller_simple64_native::settings::SavedSetup> =
                serde_json::from_str(&text)?;
            crate::controller_simple64_native::settings::validate_setups(&setups)?;
            setups
                .iter()
                .map(|setup| setup.review(&self.rust().controller_mapping.calibrations))
                .collect()
        })();
        qstring(match result {
            Ok(reviews) => serde_json::json!({"setups":reviews}).to_string(),
            Err(error) => serde_json::json!({"error":format!("{error:#}")}).to_string(),
        })
    }

    pub fn stage_simple64_native_setups(
        mut self: Pin<&mut Self>,
        configuration: QString,
    ) -> QString {
        let result =
            (|| -> anyhow::Result<Vec<crate::controller_simple64_native::settings::SavedSetup>> {
                let text = configuration.to_string();
                anyhow::ensure!(
                    text.len() <= 2 * 1024 * 1024,
                    "simple64 setup text exceeds size limit"
                );
                let setups: Vec<crate::controller_simple64_native::settings::SavedSetup> =
                    serde_json::from_str(&text)?;
                crate::controller_simple64_native::settings::validate_setups(&setups)?;
                for setup in &setups {
                    setup.review(&self.rust().controller_mapping.calibrations)?;
                }
                Ok(setups)
            })();
        match result {
            Ok(setups) => {
                self.as_mut()
                    .rust_mut()
                    .controller_mapping
                    .simple64_native_launches = setups;
                self.as_mut().controller_settings_changed();
                qstring("")
            }
            Err(error) => qstring(format!("{error:#}")),
        }
    }

    pub fn xemu_native_setups_json(&self) -> QString {
        qstring(
            serde_json::to_string_pretty(&self.rust().controller_mapping.xemu_native_launches)
                .expect("xemu_native setups serialize"),
        )
    }

    pub fn review_xemu_native_setups(&self, configuration: QString) -> QString {
        let result = (|| -> anyhow::Result<Vec<serde_json::Value>> {
            let text = configuration.to_string();
            anyhow::ensure!(
                text.len() <= 2 * 1024 * 1024,
                "xemu_native setup text exceeds size limit"
            );
            let setups: Vec<crate::controller_xemu_native::settings::SavedSetup> =
                serde_json::from_str(&text)?;
            crate::controller_xemu_native::settings::validate_setups(&setups)?;
            setups
                .iter()
                .map(|setup| setup.review(&self.rust().controller_mapping.calibrations))
                .collect()
        })();
        qstring(match result {
            Ok(reviews) => serde_json::json!({"setups":reviews}).to_string(),
            Err(error) => serde_json::json!({"error":format!("{error:#}")}).to_string(),
        })
    }

    pub fn stage_xemu_native_setups(mut self: Pin<&mut Self>, configuration: QString) -> QString {
        let result =
            (|| -> anyhow::Result<Vec<crate::controller_xemu_native::settings::SavedSetup>> {
                let text = configuration.to_string();
                anyhow::ensure!(
                    text.len() <= 2 * 1024 * 1024,
                    "xemu_native setup text exceeds size limit"
                );
                let setups: Vec<crate::controller_xemu_native::settings::SavedSetup> =
                    serde_json::from_str(&text)?;
                crate::controller_xemu_native::settings::validate_setups(&setups)?;
                for setup in &setups {
                    setup.review(&self.rust().controller_mapping.calibrations)?;
                }
                Ok(setups)
            })();
        match result {
            Ok(setups) => {
                self.as_mut()
                    .rust_mut()
                    .controller_mapping
                    .xemu_native_launches = setups;
                self.as_mut().controller_settings_changed();
                qstring("")
            }
            Err(error) => qstring(format!("{error:#}")),
        }
    }

    pub fn scummvm_native_setups_json(&self) -> QString {
        qstring(
            serde_json::to_string_pretty(&self.rust().controller_mapping.scummvm_native_launches)
                .expect("scummvm_native setups serialize"),
        )
    }

    pub fn review_scummvm_native_setups(&self, configuration: QString) -> QString {
        let result = (|| -> anyhow::Result<Vec<serde_json::Value>> {
            let text = configuration.to_string();
            anyhow::ensure!(
                text.len() <= 2 * 1024 * 1024,
                "scummvm_native setup text exceeds size limit"
            );
            let setups: Vec<crate::controller_scummvm_native::settings::SavedSetup> =
                serde_json::from_str(&text)?;
            crate::controller_scummvm_native::settings::validate_setups(&setups)?;
            setups
                .iter()
                .map(|setup| setup.review(&self.rust().controller_mapping.calibrations))
                .collect()
        })();
        qstring(match result {
            Ok(reviews) => serde_json::json!({"setups":reviews}).to_string(),
            Err(error) => serde_json::json!({"error":format!("{error:#}")}).to_string(),
        })
    }

    pub fn stage_scummvm_native_setups(
        mut self: Pin<&mut Self>,
        configuration: QString,
    ) -> QString {
        let result =
            (|| -> anyhow::Result<Vec<crate::controller_scummvm_native::settings::SavedSetup>> {
                let text = configuration.to_string();
                anyhow::ensure!(
                    text.len() <= 2 * 1024 * 1024,
                    "scummvm_native setup text exceeds size limit"
                );
                let setups: Vec<crate::controller_scummvm_native::settings::SavedSetup> =
                    serde_json::from_str(&text)?;
                crate::controller_scummvm_native::settings::validate_setups(&setups)?;
                for setup in &setups {
                    setup.review(&self.rust().controller_mapping.calibrations)?;
                }
                Ok(setups)
            })();
        match result {
            Ok(setups) => {
                self.as_mut()
                    .rust_mut()
                    .controller_mapping
                    .scummvm_native_launches = setups;
                self.as_mut().controller_settings_changed();
                qstring("")
            }
            Err(error) => qstring(format!("{error:#}")),
        }
    }
    pub fn hatari_native_setups_json(&self) -> QString {
        qstring(
            serde_json::to_string_pretty(&self.rust().controller_mapping.hatari_native_launches)
                .expect("Hatari setups serialize"),
        )
    }

    pub fn review_hatari_native_setups(&self, configuration: QString) -> QString {
        let result = (|| -> anyhow::Result<Vec<serde_json::Value>> {
            let text = configuration.to_string();
            anyhow::ensure!(
                text.len() <= 2 * 1024 * 1024,
                "Hatari setup text exceeds size limit"
            );
            let setups: Vec<crate::controller_hatari_native::settings::SavedSetup> =
                serde_json::from_str(&text)?;
            crate::controller_hatari_native::settings::validate_setups(&setups)?;
            setups
                .iter()
                .map(|setup| setup.review(&self.rust().controller_mapping.calibrations))
                .collect()
        })();
        qstring(match result {
            Ok(reviews) => serde_json::json!({"setups":reviews}).to_string(),
            Err(error) => serde_json::json!({"error":format!("{error:#}")}).to_string(),
        })
    }

    pub fn stage_hatari_native_setups(mut self: Pin<&mut Self>, configuration: QString) -> QString {
        let result =
            (|| -> anyhow::Result<Vec<crate::controller_hatari_native::settings::SavedSetup>> {
                let text = configuration.to_string();
                anyhow::ensure!(
                    text.len() <= 2 * 1024 * 1024,
                    "Hatari setup text exceeds size limit"
                );
                let setups: Vec<crate::controller_hatari_native::settings::SavedSetup> =
                    serde_json::from_str(&text)?;
                crate::controller_hatari_native::settings::validate_setups(&setups)?;
                for setup in &setups {
                    setup.review(&self.rust().controller_mapping.calibrations)?;
                }
                Ok(setups)
            })();
        match result {
            Ok(setups) => {
                self.as_mut()
                    .rust_mut()
                    .controller_mapping
                    .hatari_native_launches = setups;
                self.as_mut().controller_settings_changed();
                qstring("")
            }
            Err(error) => qstring(format!("{error:#}")),
        }
    }

    pub fn vice_native_setups_json(&self) -> QString {
        qstring(
            serde_json::to_string_pretty(&self.rust().controller_mapping.vice_native_launches)
                .expect("VICE setups serialize"),
        )
    }

    pub fn review_vice_native_setups(&self, configuration: QString) -> QString {
        let result = (|| -> anyhow::Result<Vec<serde_json::Value>> {
            let text = configuration.to_string();
            anyhow::ensure!(
                text.len() <= 2 * 1024 * 1024,
                "VICE setup text exceeds size limit"
            );
            let setups: Vec<crate::controller_vice_native::settings::SavedSetup> =
                serde_json::from_str(&text)?;
            crate::controller_vice_native::settings::validate_setups(&setups)?;
            setups
                .iter()
                .map(|setup| setup.review(&self.rust().controller_mapping.calibrations))
                .collect()
        })();
        qstring(match result {
            Ok(reviews) => serde_json::json!({"setups":reviews}).to_string(),
            Err(error) => serde_json::json!({"error":format!("{error:#}")}).to_string(),
        })
    }

    pub fn stage_vice_native_setups(mut self: Pin<&mut Self>, configuration: QString) -> QString {
        let result =
            (|| -> anyhow::Result<Vec<crate::controller_vice_native::settings::SavedSetup>> {
                let text = configuration.to_string();
                anyhow::ensure!(
                    text.len() <= 2 * 1024 * 1024,
                    "VICE setup text exceeds size limit"
                );
                let setups: Vec<crate::controller_vice_native::settings::SavedSetup> =
                    serde_json::from_str(&text)?;
                crate::controller_vice_native::settings::validate_setups(&setups)?;
                for setup in &setups {
                    setup.review(&self.rust().controller_mapping.calibrations)?;
                }
                Ok(setups)
            })();
        match result {
            Ok(setups) => {
                self.as_mut()
                    .rust_mut()
                    .controller_mapping
                    .vice_native_launches = setups;
                self.as_mut().controller_settings_changed();
                qstring("")
            }
            Err(error) => qstring(format!("{error:#}")),
        }
    }

    pub fn stella_native_setups_json(&self) -> QString {
        qstring(
            serde_json::to_string_pretty(&self.rust().controller_mapping.stella_native_launches)
                .expect("Stella setups serialize"),
        )
    }

    pub fn review_stella_native_setups(&self, configuration: QString) -> QString {
        let result = (|| -> anyhow::Result<Vec<serde_json::Value>> {
            let text = configuration.to_string();
            anyhow::ensure!(
                text.len() <= 2 * 1024 * 1024,
                "Stella setup text exceeds size limit"
            );
            let setups: Vec<crate::controller_stella_native::settings::SavedSetup> =
                serde_json::from_str(&text)?;
            crate::controller_stella_native::settings::validate_setups(&setups)?;
            setups
                .iter()
                .map(|setup| setup.review(&self.rust().controller_mapping.calibrations))
                .collect()
        })();
        qstring(match result {
            Ok(reviews) => serde_json::json!({"setups":reviews}).to_string(),
            Err(error) => serde_json::json!({"error":format!("{error:#}")}).to_string(),
        })
    }

    pub fn stage_stella_native_setups(mut self: Pin<&mut Self>, configuration: QString) -> QString {
        let result =
            (|| -> anyhow::Result<Vec<crate::controller_stella_native::settings::SavedSetup>> {
                let text = configuration.to_string();
                anyhow::ensure!(
                    text.len() <= 2 * 1024 * 1024,
                    "Stella setup text exceeds size limit"
                );
                let setups: Vec<crate::controller_stella_native::settings::SavedSetup> =
                    serde_json::from_str(&text)?;
                crate::controller_stella_native::settings::validate_setups(&setups)?;
                for setup in &setups {
                    setup.review(&self.rust().controller_mapping.calibrations)?;
                }
                Ok(setups)
            })();
        match result {
            Ok(setups) => {
                self.as_mut()
                    .rust_mut()
                    .controller_mapping
                    .stella_native_launches = setups;
                self.as_mut().controller_settings_changed();
                qstring("")
            }
            Err(error) => qstring(format!("{error:#}")),
        }
    }

    pub fn bsnes_setups_json(&self) -> QString {
        qstring(
            serde_json::to_string_pretty(&self.rust().controller_mapping.bsnes_launches)
                .expect("bsnes setups serialize"),
        )
    }

    pub fn review_bsnes_setups(&self, configuration: QString) -> QString {
        let result = (|| -> anyhow::Result<Vec<serde_json::Value>> {
            let text = configuration.to_string();
            anyhow::ensure!(
                text.len() <= 2 * 1024 * 1024,
                "bsnes setup text exceeds size limit"
            );
            let setups: Vec<crate::controller_bsnes::settings::SavedSetup> =
                serde_json::from_str(&text)?;
            crate::controller_bsnes::settings::validate_setups(&setups)?;
            setups
                .iter()
                .map(|setup| setup.review(&self.rust().controller_mapping.calibrations))
                .collect()
        })();
        qstring(match result {
            Ok(reviews) => serde_json::json!({"setups":reviews}).to_string(),
            Err(error) => serde_json::json!({"error":format!("{error:#}")}).to_string(),
        })
    }

    pub fn stage_bsnes_setups(mut self: Pin<&mut Self>, configuration: QString) -> QString {
        let result = (|| -> anyhow::Result<Vec<crate::controller_bsnes::settings::SavedSetup>> {
            let text = configuration.to_string();
            anyhow::ensure!(
                text.len() <= 2 * 1024 * 1024,
                "bsnes setup text exceeds size limit"
            );
            let setups: Vec<crate::controller_bsnes::settings::SavedSetup> =
                serde_json::from_str(&text)?;
            crate::controller_bsnes::settings::validate_setups(&setups)?;
            for setup in &setups {
                setup.review(&self.rust().controller_mapping.calibrations)?;
            }
            Ok(setups)
        })();
        match result {
            Ok(setups) => {
                self.as_mut().rust_mut().controller_mapping.bsnes_launches = setups;
                self.as_mut().controller_settings_changed();
                qstring("")
            }
            Err(error) => qstring(format!("{error:#}")),
        }
    }

    pub fn mame_native_setups_json(&self) -> QString {
        qstring(
            serde_json::to_string_pretty(&self.rust().controller_mapping.mame_native_launches)
                .expect("Standalone MAME setups serialize"),
        )
    }

    pub fn review_mame_native_setups(&self, configuration: QString) -> QString {
        let result = (|| -> anyhow::Result<Vec<serde_json::Value>> {
            let text = configuration.to_string();
            anyhow::ensure!(
                text.len() <= 2 * 1024 * 1024,
                "Standalone MAME setup text exceeds size limit"
            );
            let setups: Vec<crate::controller_mame_native::settings::SavedSetup> =
                serde_json::from_str(&text)?;
            crate::controller_mame_native::settings::validate_setups(&setups)?;
            setups
                .iter()
                .map(|setup| setup.review(&self.rust().controller_mapping.calibrations))
                .collect()
        })();
        qstring(match result {
            Ok(reviews) => serde_json::json!({"setups":reviews}).to_string(),
            Err(error) => serde_json::json!({"error":format!("{error:#}")}).to_string(),
        })
    }

    pub fn stage_mame_native_setups(mut self: Pin<&mut Self>, configuration: QString) -> QString {
        let result =
            (|| -> anyhow::Result<Vec<crate::controller_mame_native::settings::SavedSetup>> {
                let text = configuration.to_string();
                anyhow::ensure!(
                    text.len() <= 2 * 1024 * 1024,
                    "Standalone MAME setup text exceeds size limit"
                );
                let setups: Vec<crate::controller_mame_native::settings::SavedSetup> =
                    serde_json::from_str(&text)?;
                crate::controller_mame_native::settings::validate_setups(&setups)?;
                for setup in &setups {
                    setup.review(&self.rust().controller_mapping.calibrations)?;
                }
                Ok(setups)
            })();
        match result {
            Ok(setups) => {
                self.as_mut()
                    .rust_mut()
                    .controller_mapping
                    .mame_native_launches = setups;
                self.as_mut().controller_settings_changed();
                qstring("")
            }
            Err(error) => qstring(format!("{error:#}")),
        }
    }

    pub fn melonds_setups_json(&self) -> QString {
        qstring(
            serde_json::to_string_pretty(&self.rust().controller_mapping.melonds_launches)
                .expect("melonDS setups serialize"),
        )
    }

    pub fn review_melonds_setups(&self, configuration: QString) -> QString {
        let result = (|| -> anyhow::Result<Vec<serde_json::Value>> {
            let text = configuration.to_string();
            anyhow::ensure!(
                text.len() <= 2 * 1024 * 1024,
                "melonDS setup text exceeds size limit"
            );
            let setups: Vec<crate::controller_melonds::settings::SavedSetup> =
                serde_json::from_str(&text)?;
            crate::controller_melonds::settings::validate_setups(&setups)?;
            setups
                .iter()
                .map(|setup| setup.review(&self.rust().controller_mapping.calibrations))
                .collect()
        })();
        qstring(match result {
            Ok(reviews) => serde_json::json!({"setups":reviews}).to_string(),
            Err(error) => serde_json::json!({"error":format!("{error:#}")}).to_string(),
        })
    }

    pub fn stage_melonds_setups(mut self: Pin<&mut Self>, configuration: QString) -> QString {
        let result =
            (|| -> anyhow::Result<Vec<crate::controller_melonds::settings::SavedSetup>> {
                let text = configuration.to_string();
                anyhow::ensure!(
                    text.len() <= 2 * 1024 * 1024,
                    "melonDS setup text exceeds size limit"
                );
                let setups: Vec<crate::controller_melonds::settings::SavedSetup> =
                    serde_json::from_str(&text)?;
                crate::controller_melonds::settings::validate_setups(&setups)?;
                for setup in &setups {
                    setup.review(&self.rust().controller_mapping.calibrations)?;
                }
                Ok(setups)
            })();
        match result {
            Ok(setups) => {
                self.as_mut().rust_mut().controller_mapping.melonds_launches = setups;
                self.as_mut().controller_settings_changed();
                qstring("")
            }
            Err(error) => qstring(format!("{error:#}")),
        }
    }

    pub fn rpcs3_setups_json(&self) -> QString {
        qstring(
            serde_json::to_string_pretty(&self.rust().controller_mapping.rpcs3_launches)
                .expect("RPCS3 setups serialize"),
        )
    }

    pub fn review_rpcs3_setups(&self, configuration: QString) -> QString {
        let result = (|| -> anyhow::Result<Vec<serde_json::Value>> {
            let text = configuration.to_string();
            anyhow::ensure!(
                text.len() <= 2 * 1024 * 1024,
                "RPCS3 setup text exceeds size limit"
            );
            let setups: Vec<crate::controller_rpcs3::settings::SavedSetup> =
                serde_json::from_str(&text)?;
            crate::controller_rpcs3::settings::validate_setups(&setups)?;
            setups
                .iter()
                .map(|setup| setup.review(&self.rust().controller_mapping.calibrations))
                .collect()
        })();
        qstring(match result {
            Ok(reviews) => serde_json::json!({"setups":reviews}).to_string(),
            Err(error) => serde_json::json!({"error":format!("{error:#}")}).to_string(),
        })
    }

    pub fn stage_rpcs3_setups(mut self: Pin<&mut Self>, configuration: QString) -> QString {
        let result = (|| -> anyhow::Result<Vec<crate::controller_rpcs3::settings::SavedSetup>> {
            let text = configuration.to_string();
            anyhow::ensure!(
                text.len() <= 2 * 1024 * 1024,
                "RPCS3 setup text exceeds size limit"
            );
            let setups: Vec<crate::controller_rpcs3::settings::SavedSetup> =
                serde_json::from_str(&text)?;
            crate::controller_rpcs3::settings::validate_setups(&setups)?;
            for setup in &setups {
                setup.review(&self.rust().controller_mapping.calibrations)?;
            }
            Ok(setups)
        })();
        match result {
            Ok(setups) => {
                self.as_mut().rust_mut().controller_mapping.rpcs3_launches = setups;
                self.as_mut().controller_settings_changed();
                qstring("")
            }
            Err(error) => qstring(format!("{error:#}")),
        }
    }

    pub fn pcsx2_setups_json(&self) -> QString {
        qstring(
            serde_json::to_string_pretty(&self.rust().controller_mapping.pcsx2_launches)
                .expect("PCSX2 setups serialize"),
        )
    }

    pub fn review_pcsx2_setups(&self, configuration: QString) -> QString {
        let result = (|| -> anyhow::Result<Vec<serde_json::Value>> {
            let text = configuration.to_string();
            anyhow::ensure!(
                text.len() <= 2 * 1024 * 1024,
                "PCSX2 setup text exceeds size limit"
            );
            let setups: Vec<crate::controller_pcsx2::settings::SavedSetup> =
                serde_json::from_str(&text)?;
            crate::controller_pcsx2::settings::validate_setups(&setups)?;
            setups
                .iter()
                .map(|setup| setup.review(&self.rust().controller_mapping.calibrations))
                .collect()
        })();
        qstring(match result {
            Ok(reviews) => serde_json::json!({"setups":reviews}).to_string(),
            Err(error) => serde_json::json!({"error":format!("{error:#}")}).to_string(),
        })
    }

    pub fn stage_pcsx2_setups(mut self: Pin<&mut Self>, configuration: QString) -> QString {
        let result = (|| -> anyhow::Result<Vec<crate::controller_pcsx2::settings::SavedSetup>> {
            let text = configuration.to_string();
            anyhow::ensure!(
                text.len() <= 2 * 1024 * 1024,
                "PCSX2 setup text exceeds size limit"
            );
            let setups: Vec<crate::controller_pcsx2::settings::SavedSetup> =
                serde_json::from_str(&text)?;
            crate::controller_pcsx2::settings::validate_setups(&setups)?;
            for setup in &setups {
                setup.review(&self.rust().controller_mapping.calibrations)?;
            }
            Ok(setups)
        })();
        match result {
            Ok(setups) => {
                self.as_mut().rust_mut().controller_mapping.pcsx2_launches = setups;
                self.as_mut().controller_settings_changed();
                qstring("")
            }
            Err(error) => qstring(format!("{error:#}")),
        }
    }

    pub fn flycast_native_setups_json(&self) -> QString {
        qstring(
            serde_json::to_string_pretty(&self.rust().controller_mapping.flycast_native_launches)
                .expect("Standalone Flycast setups serialize"),
        )
    }

    pub fn review_flycast_native_setups(&self, configuration: QString) -> QString {
        let result = (|| -> anyhow::Result<Vec<serde_json::Value>> {
            let text = configuration.to_string();
            anyhow::ensure!(
                text.len() <= 2 * 1024 * 1024,
                "Standalone Flycast setup text exceeds size limit"
            );
            let setups: Vec<crate::controller_flycast_native::settings::SavedSetup> =
                serde_json::from_str(&text)?;
            crate::controller_flycast_native::settings::validate_setups(&setups)?;
            setups
                .iter()
                .map(|setup| setup.review(&self.rust().controller_mapping.calibrations))
                .collect()
        })();
        qstring(match result {
            Ok(reviews) => serde_json::json!({"setups":reviews}).to_string(),
            Err(error) => serde_json::json!({"error":format!("{error:#}")}).to_string(),
        })
    }

    pub fn stage_flycast_native_setups(
        mut self: Pin<&mut Self>,
        configuration: QString,
    ) -> QString {
        let result =
            (|| -> anyhow::Result<Vec<crate::controller_flycast_native::settings::SavedSetup>> {
                let text = configuration.to_string();
                anyhow::ensure!(
                    text.len() <= 2 * 1024 * 1024,
                    "Standalone Flycast setup text exceeds size limit"
                );
                let setups: Vec<crate::controller_flycast_native::settings::SavedSetup> =
                    serde_json::from_str(&text)?;
                crate::controller_flycast_native::settings::validate_setups(&setups)?;
                for setup in &setups {
                    setup.review(&self.rust().controller_mapping.calibrations)?;
                }
                Ok(setups)
            })();
        match result {
            Ok(setups) => {
                self.as_mut()
                    .rust_mut()
                    .controller_mapping
                    .flycast_native_launches = setups;
                self.as_mut().controller_settings_changed();
                qstring("")
            }
            Err(error) => qstring(format!("{error:#}")),
        }
    }

    pub fn mednafen_setups_json(&self) -> QString {
        qstring(
            serde_json::to_string_pretty(&self.rust().controller_mapping.mednafen_launches)
                .expect("Mednafen setups serialize"),
        )
    }

    pub fn review_mednafen_setups(&self, configuration: QString) -> QString {
        let result = (|| -> anyhow::Result<Vec<serde_json::Value>> {
            let text = configuration.to_string();
            anyhow::ensure!(
                text.len() <= 2 * 1024 * 1024,
                "Mednafen setup text exceeds size limit"
            );
            let setups: Vec<crate::controller_mednafen::settings::SavedSetup> =
                serde_json::from_str(&text)?;
            crate::controller_mednafen::settings::validate_setups(&setups)?;
            setups
                .iter()
                .map(|setup| setup.review(&self.rust().controller_mapping.calibrations))
                .collect()
        })();
        qstring(match result {
            Ok(reviews) => serde_json::json!({"setups":reviews}).to_string(),
            Err(error) => serde_json::json!({"error":format!("{error:#}")}).to_string(),
        })
    }

    pub fn stage_mednafen_setups(mut self: Pin<&mut Self>, configuration: QString) -> QString {
        let result =
            (|| -> anyhow::Result<Vec<crate::controller_mednafen::settings::SavedSetup>> {
                let text = configuration.to_string();
                anyhow::ensure!(
                    text.len() <= 2 * 1024 * 1024,
                    "Mednafen setup text exceeds size limit"
                );
                let setups: Vec<crate::controller_mednafen::settings::SavedSetup> =
                    serde_json::from_str(&text)?;
                crate::controller_mednafen::settings::validate_setups(&setups)?;
                for setup in &setups {
                    setup.review(&self.rust().controller_mapping.calibrations)?;
                }
                Ok(setups)
            })();
        match result {
            Ok(setups) => {
                self.as_mut()
                    .rust_mut()
                    .controller_mapping
                    .mednafen_launches = setups;
                self.as_mut().controller_settings_changed();
                qstring("")
            }
            Err(error) => qstring(format!("{error:#}")),
        }
    }

    pub fn snes9x_setups_json(&self) -> QString {
        qstring(
            serde_json::to_string_pretty(&self.rust().controller_mapping.snes9x_launches)
                .expect("Snes9x GTK setups serialize"),
        )
    }

    pub fn review_snes9x_setups(&self, configuration: QString) -> QString {
        let result = (|| -> anyhow::Result<Vec<serde_json::Value>> {
            let text = configuration.to_string();
            anyhow::ensure!(
                text.len() <= 2 * 1024 * 1024,
                "Snes9x GTK setup text exceeds size limit"
            );
            let setups: Vec<crate::controller_snes9x::settings::SavedSetup> =
                serde_json::from_str(&text)?;
            crate::controller_snes9x::settings::validate_setups(&setups)?;
            setups
                .iter()
                .map(|setup| setup.review(&self.rust().controller_mapping.calibrations))
                .collect()
        })();
        qstring(match result {
            Ok(reviews) => serde_json::json!({"setups":reviews}).to_string(),
            Err(error) => serde_json::json!({"error":format!("{error:#}")}).to_string(),
        })
    }

    pub fn stage_snes9x_setups(mut self: Pin<&mut Self>, configuration: QString) -> QString {
        let result = (|| -> anyhow::Result<Vec<crate::controller_snes9x::settings::SavedSetup>> {
            let text = configuration.to_string();
            anyhow::ensure!(
                text.len() <= 2 * 1024 * 1024,
                "Snes9x GTK setup text exceeds size limit"
            );
            let setups: Vec<crate::controller_snes9x::settings::SavedSetup> =
                serde_json::from_str(&text)?;
            crate::controller_snes9x::settings::validate_setups(&setups)?;
            for setup in &setups {
                setup.review(&self.rust().controller_mapping.calibrations)?;
            }
            Ok(setups)
        })();
        match result {
            Ok(setups) => {
                self.as_mut().rust_mut().controller_mapping.snes9x_launches = setups;
                self.as_mut().controller_settings_changed();
                qstring("")
            }
            Err(error) => qstring(format!("{error:#}")),
        }
    }

    pub fn mgba_setups_json(&self) -> QString {
        qstring(
            serde_json::to_string_pretty(&self.rust().controller_mapping.mgba_launches)
                .expect("mGBA setups serialize"),
        )
    }

    pub fn review_mgba_setups(&self, configuration: QString) -> QString {
        let result = (|| -> anyhow::Result<Vec<serde_json::Value>> {
            let text = configuration.to_string();
            anyhow::ensure!(
                text.len() <= 2 * 1024 * 1024,
                "mGBA setup text exceeds size limit"
            );
            let setups: Vec<crate::controller_mgba::settings::SavedSetup> =
                serde_json::from_str(&text)?;
            crate::controller_mgba::settings::validate_setups(&setups)?;
            setups
                .iter()
                .map(|setup| setup.review(&self.rust().controller_mapping.calibrations))
                .collect()
        })();
        qstring(match result {
            Ok(reviews) => serde_json::json!({"setups":reviews}).to_string(),
            Err(error) => serde_json::json!({"error":format!("{error:#}")}).to_string(),
        })
    }

    pub fn stage_mgba_setups(mut self: Pin<&mut Self>, configuration: QString) -> QString {
        let result = (|| -> anyhow::Result<Vec<crate::controller_mgba::settings::SavedSetup>> {
            let text = configuration.to_string();
            anyhow::ensure!(
                text.len() <= 2 * 1024 * 1024,
                "mGBA setup text exceeds size limit"
            );
            let setups: Vec<crate::controller_mgba::settings::SavedSetup> =
                serde_json::from_str(&text)?;
            crate::controller_mgba::settings::validate_setups(&setups)?;
            for setup in &setups {
                setup.review(&self.rust().controller_mapping.calibrations)?;
            }
            Ok(setups)
        })();
        match result {
            Ok(setups) => {
                self.as_mut().rust_mut().controller_mapping.mgba_launches = setups;
                self.as_mut().controller_settings_changed();
                qstring("")
            }
            Err(error) => qstring(format!("{error:#}")),
        }
    }

    pub fn ppsspp_setups_json(&self) -> QString {
        qstring(
            serde_json::to_string_pretty(&self.rust().controller_mapping.ppsspp_launches)
                .expect("PPSSPP setups serialize"),
        )
    }

    pub fn review_ppsspp_setups(&self, configuration: QString) -> QString {
        let result = (|| -> anyhow::Result<Vec<serde_json::Value>> {
            let text = configuration.to_string();
            anyhow::ensure!(
                text.len() <= 2 * 1024 * 1024,
                "PPSSPP setup text exceeds size limit"
            );
            let setups: Vec<crate::controller_ppsspp::settings::SavedSetup> =
                serde_json::from_str(&text)?;
            crate::controller_ppsspp::settings::validate_setups(&setups)?;
            setups
                .iter()
                .map(|setup| setup.review(&self.rust().controller_mapping.calibrations))
                .collect()
        })();
        qstring(match result {
            Ok(reviews) => serde_json::json!({"setups":reviews}).to_string(),
            Err(error) => serde_json::json!({"error":format!("{error:#}")}).to_string(),
        })
    }

    pub fn stage_ppsspp_setups(mut self: Pin<&mut Self>, configuration: QString) -> QString {
        let result = (|| -> anyhow::Result<Vec<crate::controller_ppsspp::settings::SavedSetup>> {
            let text = configuration.to_string();
            anyhow::ensure!(
                text.len() <= 2 * 1024 * 1024,
                "PPSSPP setup text exceeds size limit"
            );
            let setups: Vec<crate::controller_ppsspp::settings::SavedSetup> =
                serde_json::from_str(&text)?;
            crate::controller_ppsspp::settings::validate_setups(&setups)?;
            for setup in &setups {
                setup.review(&self.rust().controller_mapping.calibrations)?;
            }
            Ok(setups)
        })();
        match result {
            Ok(setups) => {
                self.as_mut().rust_mut().controller_mapping.ppsspp_launches = setups;
                self.as_mut().controller_settings_changed();
                qstring("")
            }
            Err(error) => qstring(format!("{error:#}")),
        }
    }

    pub fn stage_relative_device_settings(
        mut self: Pin<&mut Self>,
        configuration: QString,
    ) -> QString {
        let result = (|| -> anyhow::Result<Vec<crate::controller_axis::relative_settings::RelativeDeviceSettings>> {
            let text = configuration.to_string();
            anyhow::ensure!(text.len() <= 128 * 1024, "Relative device settings exceed size limit");
            let devices: Vec<crate::controller_axis::relative_settings::RelativeDeviceSettings> = serde_json::from_str(&text)?;
            crate::controller_axis::relative_settings::validate_devices(&devices)?;
            Ok(devices)
        })();
        match result {
            Ok(devices) => {
                self.as_mut().rust_mut().controller_mapping.relative_devices = devices;
                self.as_mut().controller_settings_changed();
                QString::default()
            }
            Err(error) => qstring(format!("{error:#}")),
        }
    }

    pub fn absolute_device_settings_json(&self) -> QString {
        qstring(
            serde_json::to_string_pretty(&self.rust().controller_mapping.absolute_devices)
                .expect("Absolute device settings serialize"),
        )
    }

    pub fn absolute_capture_command(mut self: Pin<&mut Self>, request: QString) -> QString {
        #[cfg(all(target_os = "linux", target_pointer_width = "64"))]
        let result = crate::controller_axis::absolute_session::command(
            &mut self.as_mut().rust_mut().absolute_capture,
            &request.to_string(),
        );
        #[cfg(not(all(target_os = "linux", target_pointer_width = "64")))]
        let result = {
            let _ = (&mut self, request);
            serde_json::json!({"error": "Absolute capture requires Linux64", "active": false, "runtime_verified": false})
        };
        qstring(serde_json::to_string(&result).expect("Absolute capture response serializes"))
    }

    pub fn stage_absolute_device_settings(
        mut self: Pin<&mut Self>,
        configuration: QString,
    ) -> QString {
        use crate::controller_axis::absolute_settings::{AbsoluteDeviceSettings, validate_devices};
        let result = (|| -> anyhow::Result<Vec<AbsoluteDeviceSettings>> {
            let text = configuration.to_string();
            anyhow::ensure!(
                text.len() <= 128 * 1024,
                "Absolute device settings exceed size limit"
            );
            let devices: Vec<AbsoluteDeviceSettings> = serde_json::from_str(&text)?;
            validate_devices(&devices)?;
            Ok(devices)
        })();
        match result {
            Ok(devices) => {
                self.as_mut().rust_mut().controller_mapping.absolute_devices = devices;
                self.as_mut().controller_settings_changed();
                QString::default()
            }
            Err(error) => qstring(format!("{error:#}")),
        }
    }

    /// Pure sample projection: neither opens hardware nor changes staged settings.
    pub fn absolute_calibration_preview_json(
        &self,
        configuration: QString,
        raw_x: i32,
        raw_y: i32,
    ) -> QString {
        use crate::controller_axis::absolute_settings::AbsoluteDeviceSettings;
        let result = (|| -> anyhow::Result<_> {
            let text = configuration.to_string();
            anyhow::ensure!(
                text.len() <= 128 * 1024,
                "Absolute device settings exceed size limit"
            );
            let device: AbsoluteDeviceSettings = serde_json::from_str(&text)?;
            Ok((
                device.project_raw(raw_x, raw_y)?,
                device.project_libretro_raw(raw_x, raw_y)?,
            ))
        })();
        let value = match result {
            Ok((projection, libretro_projection)) => serde_json::json!({
                "projection": projection,
                "libretro_projection":libretro_projection,
                "libretro_coordinate_space":"signed_minus32767_32767_without_offscreen_sentinel",
                "coordinate_space": "calibrated_unsigned_0_65535",
                "runtime_verified": false,
                "notice": "Calibration preview only; outside the calibrated area does not establish hardware offscreen or reload semantics."
            }),
            Err(error) => serde_json::json!({
                "error": format!("{error:#}"),
                "runtime_verified": false
            }),
        };
        qstring(serde_json::to_string(&value).expect("Absolute preview serializes"))
    }

    pub fn mame_dependency_discovery(&self) -> bool {
        self.rust()
            .controller_mapping
            .mame_discover_sibling_dependencies
    }

    pub fn choose_mame_dependency_discovery(mut self: Pin<&mut Self>, enabled: bool) {
        self.as_mut()
            .rust_mut()
            .controller_mapping
            .mame_discover_sibling_dependencies = enabled;
        self.as_mut().controller_settings_changed();
    }

    pub fn choose_mame_arcade_layout(mut self: Pin<&mut Self>, layout: QString) -> QString {
        use crate::controller_mame::DigitalLayout;
        let value = match layout.to_string().as_str() {
            "disabled" => None,
            "automatic" => Some(DigitalLayout::Automatic),
            "six_button" => Some(DigitalLayout::SixButton),
            "eight_button" => Some(DigitalLayout::EightButton),
            "neo_geo" => Some(DigitalLayout::NeoGeo),
            "fixed_channels" => Some(DigitalLayout::FixedChannels),
            _ => return qstring("Unknown MAME arcade layout."),
        };
        self.as_mut()
            .rust_mut()
            .controller_mapping
            .mame_arcade_layout = value;
        self.as_mut().controller_settings_changed();
        QString::default()
    }

    pub fn cancel_mame_inspection(mut self: Pin<&mut Self>) {
        if let Some(cancel) = &self.as_ref().rust().mame_inspection_cancel {
            cancel.store(true, std::sync::atomic::Ordering::Relaxed);
            self.as_mut()
                .set_mame_inspection_status(qstring("Cancelling MAME inspection…"));
        }
    }

    pub fn begin_mame_inspection(mut self: Pin<&mut Self>, request: QString) -> QString {
        if self.as_ref().rust().mame_inspection_busy {
            return qstring("MAME inspection is already running.");
        }
        let request = request.to_string();
        if request.len() > 2 * 1024 * 1024 {
            return qstring("MAME inspection request exceeds its size limit.");
        }
        let request: crate::controller_mame::InspectionRequest =
            match serde_json::from_str(&request) {
                Ok(value) => value,
                Err(error) => return qstring(format!("Invalid MAME inspection request: {error}")),
            };
        let cancel = Arc::new(AtomicBool::new(false));
        self.as_mut().rust_mut().mame_inspection_cancel = Some(cancel.clone());
        self.as_mut().rust_mut().mame_inspection_receipt = None;
        self.as_mut().set_mame_inspection_busy(true);
        self.as_mut().set_mame_inspection_result(QString::default());
        self.as_mut().set_mame_inspection_status(qstring(
            "Inspecting with the selected trusted native runtime…",
        ));
        let qt_thread = self.as_ref().qt_thread();
        let worker = std::thread::Builder::new().name("lunchbox-mame-inspection".into()).spawn(move || {
            let result = (|| -> anyhow::Result<(String, serde_json::Value)> {
                let timeout = if request.dependency_roots.is_empty() { 30 } else { 120 };
                let outcome = crate::controller_mame::inspect_runtime(&request, std::time::Duration::from_secs(timeout), &cancel)?;
                anyhow::ensure!(!cancel.load(std::sync::atomic::Ordering::Relaxed), "MAME inspection cancelled");
                let sources: Vec<_> = outcome.source_hashes.iter().map(|(path, hash)| serde_json::json!({"path":path,"sha256":hash.iter().map(|byte| format!("{byte:02x}")).collect::<String>()})).collect();
                let receipt = serde_json::json!({"core":request.core,"machine":request.machine,"requested_inputs":request.inputs,"dependency_roots":request.dependency_roots,"inputs":outcome.inputs,"snapshot":outcome.snapshot,"sources":sources});
                Ok((serde_json::to_string_pretty(&outcome.snapshot)?, receipt))
            })();
            let _ = qt_thread.queue(move |mut model| {
                model.as_mut().rust_mut().mame_inspection_cancel = None;
                model.as_mut().set_mame_inspection_busy(false);
                match result {
                    Ok((snapshot, receipt)) if !cancel.load(std::sync::atomic::Ordering::Relaxed) => {
                        let count = receipt["inputs"].as_array().map_or(0, Vec::len);
                        let discovered = receipt["dependency_roots"].as_array().is_some_and(|roots| !roots.is_empty());
                        model.as_mut().rust_mut().mame_inspection_receipt = Some(receipt);
                        model.as_mut().set_mame_inspection_result(qstring(snapshot));
                        model.as_mut().set_mame_inspection_status(qstring(format!("Native snapshot ready with {count} staged source files{}. Apply the completed inspection, then review the expanded manifest and assignments; no settings were saved.", if discovered { " after same-core dependency discovery" } else { "" })));
                    }
                    Ok(_) => model.as_mut().set_mame_inspection_status(qstring("MAME inspection cancelled. No snapshot accepted.")),
                    Err(error) => model.as_mut().set_mame_inspection_status(qstring(format!("MAME inspection stopped: {error:#}"))),
                }
            });
        });
        if let Err(error) = worker {
            self.as_mut().rust_mut().mame_inspection_cancel = None;
            self.as_mut().set_mame_inspection_busy(false);
            self.as_mut().set_mame_inspection_status(qstring(format!(
                "Could not start MAME inspection: {error}"
            )));
            return qstring(format!("Could not start MAME inspection: {error}"));
        }
        QString::default()
    }

    pub fn mame_inspection_request_json(
        &self,
        configuration: QString,
        runtime: QString,
        discovery_roots: QString,
    ) -> QString {
        let result = crate::controller_launch::mame_request_from_draft(
            &configuration.to_string(),
            std::path::Path::new(&runtime.to_string()),
            &discovery_roots.to_string(),
            &AtomicBool::new(false),
        )
        .and_then(|request| Ok(serde_json::to_string_pretty(&request)?));
        qstring(match result {
            Ok(request) => serde_json::json!({"request":request,"error":""}).to_string(),
            Err(error) => {
                serde_json::json!({"request":"","error":format!("{error:#}")}).to_string()
            }
        })
    }

    pub fn apply_mame_inspection_json(&self, configuration: QString) -> QString {
        use anyhow::Context;
        let result = (|| -> anyhow::Result<String> {
            anyhow::ensure!(
                !self.rust().mame_inspection_busy,
                "Wait for inspection to finish"
            );
            let receipt = self
                .rust()
                .mame_inspection_receipt
                .as_ref()
                .context("No completed MAME inspection is available")?;
            let text = configuration.to_string();
            anyhow::ensure!(
                text.len() <= 16 * 1024 * 1024,
                "MAME draft exceeds size limit"
            );
            let mut draft: serde_json::Value = serde_json::from_str(&text)?;
            anyhow::ensure!(draft.is_object(), "MAME draft must be an object");
            for key in ["core", "machine"] {
                anyhow::ensure!(
                    draft.get(key) == receipt.get(key),
                    "MAME draft {key} differs from the inspected request"
                );
            }
            let inspected: Vec<crate::controller_mame::InspectionInput> =
                serde_json::from_value(receipt["inputs"].clone())?;
            let declared: Vec<crate::controller_mame::InspectionInput> =
                serde_json::from_value(draft["inputs"].clone())?;
            let requested: Vec<crate::controller_mame::InspectionInput> =
                serde_json::from_value(receipt["requested_inputs"].clone())?;
            let dependency_roots: Vec<std::path::PathBuf> =
                serde_json::from_value(receipt["dependency_roots"].clone())?;
            let non_state = |inputs: &[crate::controller_mame::InspectionInput]| {
                let mut rows: Vec<_> = inputs
                    .iter()
                    .filter(|input| {
                        !input.destination.starts_with("nvram")
                            && !input.destination.starts_with("diff")
                    })
                    .cloned()
                    .collect();
                rows.sort_by(|a, b| a.destination.cmp(&b.destination));
                rows
            };
            anyhow::ensure!(
                non_state(&requested) == non_state(&declared),
                "MAME draft ROM/configuration manifest differs from the inspection request"
            );
            let inspected_non_state = non_state(&inspected);
            let requested_non_state = non_state(&requested);
            if dependency_roots.is_empty() {
                anyhow::ensure!(
                    inspected_non_state == requested_non_state,
                    "MAME explicit manifest changed during inspection"
                );
            } else {
                anyhow::ensure!(
                    requested_non_state
                        .iter()
                        .all(|input| inspected_non_state.contains(input))
                        && inspected_non_state
                            .iter()
                            .all(|input| input.destination.starts_with("roms")
                                || requested_non_state.contains(input)),
                    "MAME discovery may add ROM dependencies but cannot replace declared sources or configuration"
                );
            }
            let persistent: crate::controller_mame::PersistentPaths =
                serde_json::from_value(draft["persistent"].clone())?;
            for input in &inspected {
                for (category, root) in [("nvram", &persistent.nvram), ("diff", &persistent.diff)] {
                    if let Ok(relative) = input.destination.strip_prefix(category) {
                        anyhow::ensure!(
                            input.source == root.join(relative),
                            "Inspected state does not belong to the draft's persistent root"
                        );
                    }
                }
            }
            draft["inputs"] = receipt["inputs"].clone();
            let sources = receipt["sources"]
                .as_array()
                .context("Missing inspected source hashes")?;
            for (identity, hash) in [("core", "core_sha256"), ("content", "content_sha256")] {
                let source = sources
                    .iter()
                    .find(|source| source["path"] == draft[identity])
                    .context("MAME draft identity was not an inspected source")?;
                draft[hash] = source["sha256"].clone();
            }
            draft["reviewed_snapshot"] = receipt["snapshot"].clone();
            let setup: crate::controller_mame::NativeLaunchSettings =
                serde_json::from_value(draft)?;
            setup.validate()?;
            Ok(serde_json::to_string_pretty(&setup)?)
        })();
        qstring(match result {
            Ok(configuration) => {
                serde_json::json!({"configuration":configuration,"error":""}).to_string()
            }
            Err(error) => {
                serde_json::json!({"configuration":"","error":format!("{error:#}")}).to_string()
            }
        })
    }

    pub fn mame_digital_channels_json(&self) -> QString {
        use crate::controller_mame::AnalogChannel;
        let channels: Vec<_> = crate::controller_mame::explicit_switch_channels()
            .map(|(item, output)| {
                let axis = [
                    AnalogChannel::LeftX,
                    AnalogChannel::LeftY,
                    AnalogChannel::RightX,
                    AnalogChannel::RightY,
                    AnalogChannel::LeftPressure,
                    AnalogChannel::RightPressure,
                ]
                .into_iter()
                .find(|channel| {
                    channel
                        .controls()
                        .iter()
                        .any(|(_, channel_output)| *channel_output == output)
                });
                let opposite = axis.and_then(|channel| {
                    channel
                        .controls()
                        .iter()
                        .find(|(_, channel_output)| *channel_output != output)
                        .map(|(_, channel_output)| *channel_output)
                });
                serde_json::json!({"item":item,"output":output,
                    "axis_channel":axis,"opposite_output":opposite})
            })
            .collect();
        qstring(serde_json::to_string(&channels).expect("MAME switch channels serialize"))
    }

    pub fn mame_relative_plan_preview_json(
        &self,
        configuration: QString,
        assignments: QString,
    ) -> QString {
        let result = (|| -> anyhow::Result<_> {
            let configuration = configuration.to_string();
            let assignments = assignments.to_string();
            anyhow::ensure!(
                configuration.len() <= 16 * 1024 * 1024 && assignments.len() <= 16 * 1024 * 1024,
                "Relative preview exceeds size limit"
            );
            let setup: crate::controller_mame::NativeLaunchSettings =
                serde_json::from_str(&configuration)?;
            setup.validate_review_inputs()?;
            #[derive(serde::Deserialize)]
            #[serde(deny_unknown_fields)]
            struct WithSources {
                assignments: Vec<crate::controller_mame::RelativeAssignment>,
                sources: Vec<crate::controller_mame::RelativeSource>,
                #[serde(default)]
                buttons:
                    Option<Vec<crate::controller_mame::relative_buttons::RelativeButtonAssignment>>,
            }
            #[derive(serde::Deserialize)]
            #[serde(untagged)]
            enum Request {
                NativeOnly(Vec<crate::controller_mame::RelativeAssignment>),
                WithSources(WithSources),
            }
            let (relative, sources, buttons) = match serde_json::from_str::<Request>(&assignments)?
            {
                Request::NativeOnly(relative) => {
                    (relative, None, setup.relative_button_assignments.clone())
                }
                Request::WithSources(request) => (
                    request.assignments,
                    Some(request.sources),
                    request
                        .buttons
                        .unwrap_or_else(|| setup.relative_button_assignments.clone()),
                ),
            };
            let plan = crate::controller_mame::relative_buttons::plan_relative_button_fields(
                &setup.reviewed_snapshot,
                None,
                None,
                &setup
                    .players
                    .keys()
                    .copied()
                    .chain(relative.iter().map(|entry| entry.source_player))
                    .chain(buttons.iter().map(|entry| entry.source_player))
                    .collect(),
                &setup.analog_assignments,
                &setup.digital_assignments,
                &relative,
                &buttons,
            )?;
            let prepared_sources = sources
                .as_ref()
                .map(|sources| {
                    crate::controller_mame::relative_buttons::prepare_sources(
                        &relative, &buttons, sources,
                    )
                })
                .transpose()?;
            let profile_snapshot = crate::controller_mame::relative_buttons::gamepad_snapshot(
                &setup.reviewed_snapshot,
                &buttons,
            )?;
            let selected_players: std::collections::BTreeSet<_> = setup
                .players
                .keys()
                .copied()
                .chain(relative.iter().map(|entry| entry.source_player))
                .chain(buttons.iter().map(|entry| entry.source_player))
                .collect();
            let gamepad_requirements: Vec<_> = selected_players.iter().map(|port| {
                match crate::controller_mame::explicit_profile(
                    &profile_snapshot, &selected_players, &setup.analog_assignments,
                    &setup.digital_assignments, *port, setup.digital_layout_for(*port),
                ) {
                    Ok(profile) => serde_json::json!({"port":port,
                        "gamepad_required":profile.is_some(),
                        "gamepad_selected":setup.players.contains_key(port),
                        "error":""}),
                    Err(error) => serde_json::json!({"port":port,"gamepad_required":null,
                        "gamepad_selected":setup.players.contains_key(port),"error":format!("{error:#}")}),
                }
            }).collect();
            let game_mouse_mode_error = if buttons.is_empty() {
                None
            } else {
                setup
                    .reviewed_snapshot
                    .require_game_mouse_mode()
                    .err()
                    .map(|error| format!("{error:#}"))
            };
            let relative_platform_pending = (!relative.is_empty() || !buttons.is_empty())
                && !cfg!(all(target_os = "linux", target_pointer_width = "64"));
            let button_mapping_pending = !buttons.is_empty()
                && (relative_platform_pending || game_mouse_mode_error.is_some());
            // Review does not open devices or prove launch readiness. Staging
            // and fresh launch inspection independently enforce runtime requirements.
            let draft_configuration = if let Some(sources) = &sources {
                let mut candidate = setup.clone();
                candidate.relative_assignments = relative.clone();
                candidate.relative_button_assignments = buttons.clone();
                candidate.relative_sources = sources.clone();
                candidate.validate_review_inputs()?;
                Some(serde_json::to_string_pretty(&candidate)?)
            } else {
                None
            };
            Ok(
                serde_json::json!({"controller_xml":plan.controller_xml,"game_config_xml":plan.game_config_xml,
                "mapped_native_fields":plan.mapped_fields,"unhandled":plan.unhandled_fields,
                "saved_sources_validated":sources.is_some(),
                "prepared_sources":prepared_sources,
                "button_assignments":buttons,
                "button_mapping_pending":button_mapping_pending,
                "game_mouse_mode_error":game_mouse_mode_error,
                "gamepad_requirements":gamepad_requirements,
                "apply_blocked_reason":if sources.is_some() { "" } else { "Native-only previews have no validated source set and cannot be applied." },
                "launch_blocked_reason":if relative_platform_pending {
                    "Physical MAME relative input requires 64-bit Linux."
                } else if button_mapping_pending {
                    "Mouse-button mapping requires a fresh inspection declaring the opt-in game-mouse core mode."
                } else { "" },
                "draft_configuration":draft_configuration,
                "launch_enabled":false,"physical_routing":false,"error":"",
                "warning":"Draft replacement preview only; nothing was saved or captured. Physical-relative launch requires the opt-in frontend routing extension on 64-bit Linux. Gamepad-backed ports require usable profiles; relative-only ports must have no remaining gamepad-channel requirements. Runtime behavior remains unverified."}),
            )
        })();
        qstring(match result {
            Ok(value) => value.to_string(),
            Err(error) => serde_json::json!({"error":format!("{error:#}"),"launch_enabled":false,"physical_routing":false}).to_string(),
        })
    }

    pub fn mame_assignment_review_json(&self, configuration: QString) -> QString {
        use anyhow::Context;
        let result = (|| -> anyhow::Result<_> {
            let text = configuration.to_string();
            anyhow::ensure!(
                text.len() <= 16 * 1024 * 1024,
                "MAME setup exceeds review limit"
            );
            let setup: crate::controller_mame::NativeLaunchSettings = serde_json::from_str(&text)?;
            setup.validate_review_inputs()?;
            let mut players = Vec::new();
            let profile_snapshot = crate::controller_mame::relative_buttons::gamepad_snapshot(
                &setup.reviewed_snapshot,
                &setup.relative_button_assignments,
            )?;
            let mut analog_calibrations = Vec::new();
            let relative_platform_pending = (!setup.relative_assignments.is_empty()
                || !setup.relative_button_assignments.is_empty())
                && !cfg!(all(target_os = "linux", target_pointer_width = "64"));
            let button_mapping_pending = !setup.relative_button_assignments.is_empty()
                && (relative_platform_pending
                    || setup.reviewed_snapshot.require_game_mouse_mode().is_err());
            let mut physical_pending = relative_platform_pending || button_mapping_pending;
            // Informational only: successful native resolution does not bind
            // physical devices or establish frontend startup indices.
            let mouse_routes = setup.reviewed_snapshot.mouse_routes();
            let mut native_mouse_routing = match &mouse_routes {
                Ok(routes) => {
                    serde_json::json!({"native_sequence_indices":routes,"error":"","physical_routing":false})
                }
                Err(error) => {
                    serde_json::json!({"native_sequence_indices":null,"error":format!("{error:#}"),"physical_routing":false})
                }
            };
            let analog_state = |field: &crate::controller_mame::ActiveField| {
                setup
                    .reviewed_snapshot
                    .analog_states
                    .iter()
                    .find(|state| state.field == *field)
            };
            let mut relative_only_ports = Vec::new();
            let selected = setup.selected_players();
            for port in selected
                .iter()
                .filter(|port| !setup.players.contains_key(port))
            {
                let error = match crate::controller_mame::explicit_profile(
                    &profile_snapshot,
                    &selected,
                    &setup.analog_assignments,
                    &setup.digital_assignments,
                    *port,
                    crate::controller_mame::DigitalLayout::Automatic,
                ) {
                    Ok(None) => String::new(),
                    Ok(Some(_)) => format!(
                        "Player {port} still requires gamepad channels; assign a gamepad or reroute those actions"
                    ),
                    Err(error) => format!("{error:#}"),
                };
                physical_pending |= !error.is_empty();
                relative_only_ports.push(serde_json::json!({"port":port,"error":error}));
            }
            native_mouse_routing["relative_only_ports"] = serde_json::json!(relative_only_ports);
            native_mouse_routing["button_assignments"] =
                serde_json::json!(setup.relative_button_assignments);
            native_mouse_routing["button_mapping_pending"] =
                serde_json::json!(button_mapping_pending);
            native_mouse_routing["platform_error"] =
                serde_json::json!(if relative_platform_pending {
                    "Physical MAME relative input requires 64-bit Linux."
                } else {
                    ""
                });
            native_mouse_routing["game_mouse_mode"] = match setup
                .reviewed_snapshot
                .require_game_mouse_mode()
            {
                Ok(()) => serde_json::json!({"declared":true,"error":"","runtime_verified":false}),
                Err(error) => {
                    serde_json::json!({"declared":false,"error":format!("{error:#}"),"runtime_verified":false})
                }
            };
            let relative_routes: Vec<_> = setup.relative_assignments.iter().map(|assignment| {
                let source = setup.relative_sources.iter()
                    .find(|source| source.source_player == assignment.source_player)
                    .context("Relative mapping has no saved source")?;
                let motion = source.device.motion;
                let output = assignment.output_axis;
                let physical_axis = if motion.swap_xy { output ^ 1 } else { output };
                let (sensitivity_percent, inverted) = if output == 0 {
                    (motion.x_percent, motion.invert_x)
                } else {
                    (motion.y_percent, motion.invert_y)
                };
                Ok(serde_json::json!({"assignment":assignment,
                    "event_path":source.device.event_path,"input_identity":source.device.input_identity,
                    "physical_axis":physical_axis,"sensitivity_percent":sensitivity_percent,
                    "inverted":inverted,"runtime_verified":false}))
            }).collect::<anyhow::Result<_>>()?;
            native_mouse_routing["routes"] = serde_json::json!(relative_routes);
            let button_routes: Vec<_> = setup.relative_button_assignments.iter().map(|assignment| {
                let source = setup.relative_sources.iter()
                    .find(|source| source.source_player == assignment.source_player)
                    .context("Mouse-button mapping has no saved source")?;
                let output_code = crate::controller_mame::relative_buttons::output_evdev_code(
                    assignment.output_button,
                )?;
                let (physical_button, _) = source.device.buttons.iter()
                    .find(|(_, output)| *output == output_code)
                    .context("Mouse-button mapping has no physical button for its output")?;
                Ok(serde_json::json!({"kind":"button","assignment":assignment,
                    "event_path":source.device.event_path,"input_identity":source.device.input_identity,
                    "physical_button":physical_button,"output_code":output_code,
                    "runtime_verified":false}))
            }).collect::<anyhow::Result<_>>()?;
            native_mouse_routing["button_routes"] = serde_json::json!(button_routes);
            let field_label = |field: &crate::controller_mame::ActiveField| {
                setup
                    .reviewed_snapshot
                    .field_labels
                    .iter()
                    .find(|entry| entry.field == *field)
                    .map(|entry| entry.label.as_str())
                    .unwrap_or("")
            };
            let switch_routes = crate::controller_mame::resolved_switch_routes(
                &profile_snapshot,
                &setup.selected_players(),
                &setup.analog_assignments,
                &setup.digital_assignments,
            )?;
            for (port, controller) in &setup.players {
                // Semantic routes do not require a valid physical calibration.
                // Keep them separate from drawable, measured mapping rows.
                let switch_actions: Vec<_> = switch_routes
                    .iter()
                    .filter(|route| route.assignment.source_player == *port)
                    .map(|route| {
                        serde_json::json!({"assignment":route.assignment,
                            "explicit":route.explicit,"label":field_label(&route.assignment.field)})
                    })
                    .collect();
                let profile = match crate::controller_mame::explicit_profile(
                    &profile_snapshot,
                    &setup.selected_players(),
                    &setup.analog_assignments,
                    &setup.digital_assignments,
                    *port,
                    setup.digital_layout_for(*port),
                ) {
                    Ok(profile) => profile,
                    Err(error) => {
                        physical_pending = true;
                        players.push(serde_json::json!({
                            "port":port,"controller":controller,
                            "layout":"Destination layout unavailable",
                            "source_layout":null,"target_layout":null,
                            "rows":[],"native_routes":[],"physical_gaps":[],
                            "switch_actions":switch_actions,
                            "layout_error":format!("{error:#}"),
                            "warnings":[format!("{error:#}. Choose a compatible preset or revise this player's assignments, then review again. No physical mapping was produced; staging remains blocked.")]
                        }));
                        continue;
                    }
                };
                let mut validated_source_layout = None;
                let calibration = self
                    .rust()
                    .controller_mapping
                    .calibrations
                    .get(controller)
                    .context("MAME player has no saved calibration")
                    .and_then(|calibration| {
                        calibration.validate()?;
                        validated_source_layout = Some(calibration.layout.as_str());
                        let plan = profile
                            .as_ref()
                            .map(|profile| calibration.plan_profile(profile))
                            .transpose()?;
                        Ok((calibration, plan))
                    });
                let (calibration, physical_plan) = match calibration {
                    Ok(prepared) => prepared,
                    Err(error) => {
                        physical_pending = true;
                        let mut rows = Vec::new();
                        let mut native_routes = Vec::new();
                        if let Some(profile) = &profile {
                            let layout = crate::controller_catalog::catalog()
                                .layout(&profile.target_layout)
                                .context("MAME destination layout is unavailable")?;
                            for control in &layout.controls {
                                let Some(output) = profile.bindings.get(&control.id) else {
                                    continue;
                                };
                                rows.push(crate::controller_catalog::MappingRow {
                                    target_id: control.id.clone(),
                                    target: control.label.clone(),
                                    physical_id: None,
                                    physical: "UNMAPPED".to_owned(),
                                    input: None,
                                    output: output.clone(),
                                    reason: "Game-side requirement only; physical calibration is unavailable".to_owned(),
                                });
                                for route in switch_routes.iter().filter(|route| {
                                    route.assignment.source_player == *port
                                        && route.assignment.output == *output
                                }) {
                                    native_routes.push(serde_json::json!({"target_id":control.id,
                                        "assignment":route.assignment,"explicit":route.explicit,
                                        "label":field_label(&route.assignment.field),
                                        "analog_state":analog_state(&route.assignment.field)}));
                                }
                                for assignment in
                                    setup.analog_assignments.iter().filter(|assignment| {
                                        assignment.source_player == *port
                                            && assignment
                                                .channel
                                                .controls()
                                                .iter()
                                                .any(|(target, _)| *target == control.id)
                                    })
                                {
                                    native_routes.push(serde_json::json!({"target_id":control.id,
                                        "assignment":assignment,"label":field_label(&assignment.field),
                                        "analog_state":analog_state(&assignment.field)}));
                                }
                            }
                        }
                        let physical_gaps: Vec<_> = rows
                            .iter()
                            .map(|row| {
                                serde_json::json!({"target_id":row.target_id,"target":row.target,
                                "physical_id":null,"reason":"Physical calibration is unavailable"})
                            })
                            .collect();
                        let mut warnings = profile
                            .as_ref()
                            .map(|profile| profile.conditions.clone())
                            .unwrap_or_default();
                        warnings.push(format!("{error:#}. Change this player's controller or repair its calibration before staging."));
                        if validated_source_layout.is_some() {
                            warnings.push("The source outline comes from valid saved calibration, but no complete physical plan was produced. Displayed connections remain unmapped; no device has been verified.".to_owned());
                        }
                        players.push(serde_json::json!({"port":port,"controller":controller,
                            "layout":"Physical mapping unavailable","source_layout":validated_source_layout,
                            "target_layout":profile.as_ref().map(|profile| &profile.target_layout),
                            "rows":rows,"native_routes":native_routes,"physical_gaps":physical_gaps,
                            "switch_actions":switch_actions,"calibration_error":format!("{error:#}"),
                            "warnings":warnings}));
                        continue;
                    }
                };
                if let Some(profile) = profile {
                    let plan = physical_plan.context("MAME physical plan was not retained")?;
                    let physical_gaps: Vec<_> = plan
                        .rows
                        .iter()
                        .filter(|row| {
                            !row.input
                                .as_ref()
                                .is_some_and(|input| input.native.is_some())
                        })
                        .map(|row| {
                            serde_json::json!({"target_id":row.target_id,
                        "target":row.target,"physical_id":row.physical_id,
                        "reason":if row.input.is_none() { "No calibrated source assignment" }
                            else { "Source calibration has no native input measurement" }})
                        })
                        .collect();
                    physical_pending |= !physical_gaps.is_empty();
                    // Count the same required rows as the staging guard. A
                    // semantic source assignment alone is not a measurement.
                    let required_measurements = plan.rows.len();
                    let measured = required_measurements - physical_gaps.len();
                    let measurement_coverage = serde_json::json!({
                        "required": required_measurements,
                        "measured": measured,
                        "missing": physical_gaps.len(),
                        "percent": if required_measurements == 0 { None } else {
                            Some(100.0 * measured as f64 / required_measurements as f64)
                        },
                        "runtime_verified": false,
                    });
                    let mut warnings = plan.warnings.clone();
                    for gap in &physical_gaps {
                        warnings.push(format!("{} [{}]: {}. Repair this player's calibration or choose another controller before staging.",
                            gap["target"].as_str().unwrap_or("Control"),
                            gap["target_id"].as_str().unwrap_or(""),
                            gap["reason"].as_str().unwrap_or("Physical mapping unavailable")));
                    }
                    if setup
                        .analog_assignments
                        .iter()
                        .any(|assignment| assignment.source_player == *port)
                    {
                        let error = crate::controller_mame::normalized_calibration(
                            &setup.analog_assignments,
                            *port,
                            calibration,
                            &profile,
                        )
                        .err()
                        .map(|error| format!("{error:#}"))
                        .unwrap_or_default();
                        analog_calibrations.push(serde_json::json!({"port":port,"rows":[],"warnings":["Analog checks apply to the combined mapping rows above; live input is not verified."],"error":error}));
                    }
                    // Use the same channel/control identities as profile generation,
                    // including multiple native fields driven by one source axis.
                    let mut analog_routes = Vec::new();
                    for assignment in setup
                        .analog_assignments
                        .iter()
                        .filter(|assignment| assignment.source_player == *port)
                    {
                        for (target, _) in assignment.channel.controls() {
                            analog_routes.push(serde_json::json!({
                                "target_id": target,
                                "assignment": assignment,
                                "label": field_label(&assignment.field),
                                "analog_state": analog_state(&assignment.field),
                            }));
                        }
                    }
                    let mut native_routes = analog_routes;
                    for route in switch_routes
                        .iter()
                        .filter(|route| route.assignment.source_player == *port)
                    {
                        let assignment = &route.assignment;
                        for row in plan
                            .rows
                            .iter()
                            .filter(|row| row.output == assignment.output)
                        {
                            native_routes.push(serde_json::json!({"target_id":row.target_id,"assignment":assignment,"explicit":route.explicit,"label":field_label(&assignment.field),"analog_state":analog_state(&assignment.field)}));
                        }
                    }
                    players.push(serde_json::json!({"port":port,"controller":controller,"layout":profile.name,"source_layout":calibration.layout,"target_layout":profile.target_layout,"rows":plan.rows,"native_routes":native_routes,"switch_actions":switch_actions,"physical_gaps":physical_gaps,"measurement_coverage":measurement_coverage,"warnings":warnings}));
                } else {
                    physical_pending = true;
                    players.push(serde_json::json!({"port":port,"controller":controller,"layout":"No active mapped controls","rows":[],"switch_actions":switch_actions,"warnings":["This selected source player has no active mapping profile. Remove the unused port or configure its inspected controls before staging; no physical mapping is established."]}));
                }
            }
            let ports = setup.selected_players();
            // Review must expose even a zero-mapping draft so its unresolved
            // fields can be assigned. The explicit planner handles empty lists;
            // staging and launch retain their separate completeness guards.
            let native = crate::controller_mame::relative_buttons::plan_relative_button_fields(
                &setup.reviewed_snapshot,
                None,
                None,
                &ports,
                &setup.analog_assignments,
                &setup.digital_assignments,
                &setup.relative_assignments,
                &setup.relative_button_assignments,
            )?;
            let analog_pending = analog_calibrations.iter().any(|entry| {
                entry["error"]
                    .as_str()
                    .is_none_or(|error| !error.is_empty())
            });
            let calibration_pending = players
                .iter()
                .any(|player| player.get("calibration_error").is_some());
            let mut generated_layouts = 0usize;
            let mut failed_layouts = 0usize;
            let mut inactive_layouts = 0usize;
            for player in &players {
                if player.get("layout_error").is_some() {
                    failed_layouts += 1;
                } else if player
                    .get("target_layout")
                    .and_then(|value| value.as_str())
                    .is_some()
                {
                    generated_layouts += 1;
                } else {
                    inactive_layouts += 1;
                }
            }
            anyhow::ensure!(
                players.len() == setup.players.len(),
                "MAME selected-player layout accounting is incomplete"
            );
            let layout_coverage = serde_json::json!({
                "selected_players":players.len(),"generated":generated_layouts,
                "failed":failed_layouts,"no_active_profile":inactive_layouts,
                "generated_percent":if players.is_empty() { None } else {
                    Some(100.0 * generated_layouts as f64 / players.len() as f64)
                }
            });
            let user_fields = native.mapped_fields
                + native.disabled_fields
                + native.unhandled_fields.len()
                + native.preserved_service.len();
            anyhow::ensure!(
                user_fields + native.preserved_settings.len() + native.preserved_internal.len()
                    == setup.reviewed_snapshot.fields.len(),
                "MAME field coverage accounting is inconsistent"
            );
            // Keep the full user-input denominator. The additional assignment
            // denominator excludes intentional NONE routes and native service
            // bindings, but never excludes unresolved input contracts.
            let assignment_fields = native.mapped_fields + native.unhandled_fields.len();
            let field_coverage = serde_json::json!({"user_fields":user_fields,
                "mapped":native.mapped_fields,"disabled":native.disabled_fields,"unresolved":native.unhandled_fields.len(),
                "native_service":native.preserved_service.len(),
                "assignment_fields":assignment_fields,
                "assignment_percent":if assignment_fields == 0 { None } else {
                    Some(100.0 * native.mapped_fields as f64 / assignment_fields as f64)
                },
                "mapped_percent":if user_fields == 0 { None } else { Some(100.0 * native.mapped_fields as f64 / user_fields as f64) }});
            // A per-field match alone can propose incompatible devices for
            // the two axes of one player. Require their complete unresolved
            // relative-axis set while retaining every exact native field.
            let mut relative_player_axes: std::collections::BTreeMap<
                usize,
                std::collections::BTreeSet<u16>,
            > = Default::default();
            for field in &native.unhandled_fields {
                if let Some((player, axis)) =
                    crate::controller_mame::relative_axis_requirement(field)
                {
                    relative_player_axes.entry(player).or_default().insert(axis);
                }
            }
            let unresolved_controls: Vec<_> = native
                .unhandled_fields
                .iter()
                .map(|field| {
                    let relative_input = crate::controller_mame::relative_axis_requirement(field)
                        .map(|(player, axis)| {
                            let required_axes = &relative_player_axes[&player];
                            let sequence = if !setup.players.contains_key(&player) {
                                Err(anyhow::anyhow!("Native player is not selected as a source"))
                            } else {
                                match &mouse_routes {
                                    Ok(routes) => crate::controller_mame::relative_axis_sequence(field, axis, routes[player - 1]),
                                    Err(error) => Err(anyhow::anyhow!("{error:#}")),
                                }
                            };
                            let (native_sequence, native_error) = match sequence {
                                Ok(sequence) => (Some(sequence), String::new()),
                                Err(error) => (None, format!("{error:#}")),
                            };
                            let candidates: Vec<_> = self
                                .rust()
                                .controller_mapping
                                .relative_devices
                                .iter()
                                .enumerate()
                                .filter(|(_, device)| {
                                    required_axes.iter().all(|required_axis|
                                        device.supports_output(2, *required_axis).unwrap_or(false))
                                })
                                .map(|(index, device)| {
                                    serde_json::json!({
                                        "saved_device_index":index,"event_path":device.event_path,
                                        "input_identity":device.input_identity
                                    })
                                })
                                .collect();
                            serde_json::json!({"native_player":player,"event_type":2,
                                "suggested_output_axis":axis,"saved_device_candidates":candidates,
                                "required_player_axes":required_axes,
                                "suggested_native_sequence":native_sequence,"native_error":native_error,
                                "mapped":false,"launch_enabled":false})
                        });
                    serde_json::json!({"field":field,"label":field_label(field),
                    "relative_input":relative_input,
                    "guidance":crate::controller_mame::unresolved_field_guidance(field)})
                })
                .collect();
            Ok(
                serde_json::json!({"players":players,"native_mouse_routing":native_mouse_routing,"layout_coverage":layout_coverage,"physical_pending":physical_pending,"calibration_pending":calibration_pending,"field_coverage":field_coverage,"unresolved_controls":unresolved_controls,"analog_calibrations":analog_calibrations,"analog_assignments":setup.analog_assignments,"digital_assignments":setup.digital_assignments,"analog_pending":analog_pending,"unhandled":native.unhandled_fields,"preserved_settings":native.preserved_settings,"preserved_internal":native.preserved_internal,"preserved_service":native.preserved_service,"error":"","launch_ready":false}),
            )
        })();
        qstring(match result {
            Ok(value) => value.to_string(),
            Err(error) => serde_json::json!({"players":[],"unhandled":[],"error":format!("{error:#}"),"launch_ready":false}).to_string(),
        })
    }

    pub fn mame_controller_setup_json(&self, key: QString) -> QString {
        let key = key.to_string();
        self.rust()
            .controller_mapping
            .mame_launches
            .iter()
            .find(|setup| setup.identity_key() == key)
            .map(|setup| {
                qstring(serde_json::to_string_pretty(setup).expect("MAME setup serializes"))
            })
            .unwrap_or_default()
    }

    pub fn save_mame_controller_setup(mut self: Pin<&mut Self>, configuration: QString) -> QString {
        use anyhow::Context;
        let result = (|| -> anyhow::Result<_> {
            let text = configuration.to_string();
            anyhow::ensure!(
                text.len() <= 16 * 1024 * 1024,
                "MAME setup exceeds its size limit"
            );
            let setup: crate::controller_mame::NativeLaunchSettings = serde_json::from_str(&text)?;
            setup.validate()?;
            let mut mapping = self.as_ref().rust().controller_mapping.clone();
            let ports = setup.selected_players();
            let profile_snapshot = crate::controller_mame::relative_buttons::gamepad_snapshot(
                &setup.reviewed_snapshot,
                &setup.relative_button_assignments,
            )?;
            let native = if !setup.relative_assignments.is_empty()
                || !setup.relative_button_assignments.is_empty()
            {
                crate::controller_mame::relative_buttons::plan_relative_button_fields(
                    &setup.reviewed_snapshot,
                    None,
                    None,
                    &ports,
                    &setup.analog_assignments,
                    &setup.digital_assignments,
                    &setup.relative_assignments,
                    &setup.relative_button_assignments,
                )?
            } else if !setup.digital_assignments.is_empty() {
                crate::controller_mame::plan_explicit_fields(
                    &setup.reviewed_snapshot,
                    None,
                    None,
                    &ports,
                    &setup.analog_assignments,
                    &setup.digital_assignments,
                )?
            } else if setup.analog_assignments.is_empty() {
                crate::controller_mame::plan_digital_fields(
                    &setup.reviewed_snapshot,
                    None,
                    None,
                    &ports,
                )?
            } else {
                crate::controller_mame::plan_mixed_fields(
                    &setup.reviewed_snapshot,
                    None,
                    None,
                    &ports,
                    &setup.analog_assignments,
                )?
            };
            anyhow::ensure!(
                native.unhandled_fields.is_empty(),
                "MAME setup has {} unresolved native fields. Their input contracts are required before staging a launch setup.",
                native.unhandled_fields.len()
            );
            for (port, controller) in &setup.players {
                let calibration = mapping
                    .calibrations
                    .get(controller)
                    .context("MAME player lacks saved physical calibration")?;
                let profile = crate::controller_mame::explicit_profile(
                    &profile_snapshot,
                    &setup.selected_players(),
                    &setup.analog_assignments,
                    &setup.digital_assignments,
                    *port,
                    setup.digital_layout_for(*port),
                )?
                .context("Selected MAME player has no active mapped controls")?;
                let plan = calibration.plan_profile(&profile)?;
                if setup
                    .analog_assignments
                    .iter()
                    .any(|assignment| assignment.source_player == *port)
                {
                    crate::controller_mame::normalized_calibration(
                        &setup.analog_assignments,
                        *port,
                        calibration,
                        &profile,
                    )?;
                }
                anyhow::ensure!(
                    plan.rows.iter().all(|row| row
                        .input
                        .as_ref()
                        .is_some_and(|input| input.native.is_some())),
                    "MAME player has unmapped or unmeasured physical controls"
                );
            }
            let key = setup.identity_key();
            mapping
                .mame_launches
                .retain(|previous| previous.identity_key() != key);
            mapping.mame_launches.push(setup);
            mapping.validate()?;
            Ok(mapping)
        })();
        match result {
            Ok(mapping) => {
                self.as_mut().rust_mut().controller_mapping = mapping;
                self.as_mut().controller_settings_changed();
                self.as_mut().set_message(qstring("MAME setup staged. Save settings to keep it. No runtime or device was opened; native fields and physical inputs will be checked at launch."));
                QString::default()
            }
            Err(error) => qstring(format!("Invalid MAME setup: {error:#}")),
        }
    }

    pub fn remove_mame_controller_setup(mut self: Pin<&mut Self>, key: QString) -> QString {
        let key = key.to_string();
        let mut mapping = self.as_ref().rust().controller_mapping.clone();
        let count = mapping.mame_launches.len();
        mapping
            .mame_launches
            .retain(|setup| setup.identity_key() != key);
        if mapping.mame_launches.len() == count {
            return qstring("That MAME setup no longer exists. Refresh the list.");
        }
        self.as_mut().rust_mut().controller_mapping = mapping;
        self.as_mut().controller_settings_changed();
        self.as_mut().set_message(qstring("MAME setup removed from staged settings. Save settings to keep this change. Game files and calibrations are unchanged."));
        QString::default()
    }

    /// Saved identities only: enumerating these choices does not open hardware
    /// or claim that a controller is currently connected or launch-ready.
    pub fn fbneo_source_controllers_json(&self) -> QString {
        let mut rows = self.rust().controller_mapping.calibrations.iter()
            .filter_map(|(id, calibration)| {
                if calibration.os != "linux" || calibration.validate().is_err() {
                    return None;
                }
                let layout = crate::controller_catalog::catalog().layout(&calibration.layout)?;
                Some(serde_json::json!({"id": id, "layout": layout.name,
                    "native_bindings": calibration.bindings.values().filter(|binding| binding.native.is_some()).count()}))
            }).collect::<Vec<_>>();
        rows.sort_by(|left, right| left["id"].as_str().cmp(&right["id"].as_str()));
        qstring(serde_json::to_string(&rows).expect("Saved FBNeo controller choices serialize"))
    }

    pub fn fbneo_device_choices_json(&self, context: QString) -> QString {
        let result = (|| -> anyhow::Result<Vec<serde_json::Value>> {
            let text = context.to_string();
            anyhow::ensure!(
                text.len() <= 1024 * 1024,
                "FBNeo context exceeds its size limit"
            );
            let context: crate::controller_fbneo::InspectionImportContext =
                serde_json::from_str(&text)?;
            let topology = crate::controller_fbneo::controller_topology(
                context.hardware,
                context.driver_players,
                context.mahjong_keyboards,
            )?;
            Ok(topology
                .advertised
                .iter()
                .enumerate()
                .map(|(port, devices)| {
                    serde_json::json!({"port": port, "devices": devices.iter().map(|device|
                    serde_json::json!({"id": device.libretro_id(), "label": format!("{device:?}")}))
                    .collect::<Vec<_>>()})
                })
                .collect())
        })();
        qstring(match result {
            Ok(ports) => serde_json::json!({"ports": ports, "error": ""}).to_string(),
            Err(error) => {
                serde_json::json!({"ports": [], "error": format!("{error:#}")}).to_string()
            }
        })
    }

    pub fn cancel_fbneo_inspection(mut self: Pin<&mut Self>) {
        if let Some(cancel) = &self.as_ref().rust().fbneo_inspection_cancel {
            cancel.store(true, std::sync::atomic::Ordering::Relaxed);
            self.as_mut().set_fbneo_import_status(qstring(
                "Cancelling FBNeo inspection and cleaning its private workspace…",
            ));
        }
    }

    pub fn begin_fbneo_inspection(
        mut self: Pin<&mut Self>,
        request: QString,
        context: QString,
    ) -> QString {
        if self.as_ref().rust().fbneo_import_busy {
            return qstring("An FBNeo inspection or import is already running.");
        }
        let request = request.to_string();
        let context = context.to_string();
        if request.len() > 1024 * 1024 || context.len() > 64 * 1024 {
            return qstring("FBNeo inspection input exceeds its size limit.");
        }
        let cancel = Arc::new(AtomicBool::new(false));
        self.as_mut().rust_mut().fbneo_inspection_cancel = Some(cancel.clone());
        self.as_mut().set_fbneo_import_busy(true);
        self.as_mut().set_fbneo_inspection_active(true);
        self.as_mut().set_fbneo_import_draft(QString::default());
        self.as_mut().set_fbneo_import_status(qstring(
            "Running the explicitly selected trusted core in an isolated inspection worker…",
        ));
        let qt_thread = self.as_ref().qt_thread();
        let worker = std::thread::Builder::new().name("lunchbox-fbneo-inspection".into()).spawn(move || {
            let result = (|| -> anyhow::Result<String> {
                let request: lunchbox_controller_probe::content_inspection::Request = serde_json::from_str(&request)?;
                request.validate()?;
                let context: crate::controller_fbneo::InspectionImportContext = serde_json::from_str(&context)?;
                anyhow::ensure!(!context.emulator_id.is_empty() && context.emulator_id.len() <= 1024,
                    "Invalid inspection emulator identity");
                let topology = crate::controller_fbneo::controller_topology(context.hardware,
                    context.driver_players, context.mahjong_keyboards)?;
                let ports = request.devices.iter().map(|device| device.port).collect::<std::collections::BTreeSet<_>>();
                anyhow::ensure!(ports.len() == topology.advertised.len()
                    && ports.iter().eq(context.controllers.keys()), "Select every inspected port's physical controller");
                let mut ids = std::collections::BTreeSet::new();
                for id in context.controllers.values() {
                    anyhow::ensure!(!id.is_empty() && id.len() <= 4096 && ids.insert(id),
                        "Invalid or duplicate inspection physical controller");
                }
                for device in &request.devices {
                    anyhow::ensure!(topology.advertised.get(device.port as usize).is_some_and(|choices|
                        choices.iter().any(|choice| choice.libretro_id() == device.device)),
                        "Inspection device differs from the selected topology");
                }
                let report = crate::controller_fbneo::inspect_runtime(&context.helper, &[], &request,
                    &topology, std::time::Duration::from_secs(30), &cancel)?;
                let draft = crate::controller_fbneo::import_inspection(&request,
                    &serde_json::to_vec(&report)?, context)?;
                anyhow::ensure!(!cancel.load(std::sync::atomic::Ordering::Relaxed), "FBNeo inspection cancelled");
                Ok(serde_json::to_string_pretty(&draft)?)
            })();
                let _ = qt_thread.queue(move |mut model| {
                    let cancelled = cancel.load(std::sync::atomic::Ordering::Relaxed);
                    model.as_mut().rust_mut().fbneo_inspection_cancel = None;
                model.as_mut().set_fbneo_inspection_active(false);
                model.as_mut().set_fbneo_import_busy(false);
                match result {
                        Ok(draft) if !cancelled => {
                        model.as_mut().set_fbneo_import_draft(qstring(draft));
                            model.as_mut().set_fbneo_import_status(qstring("Inspection draft ready. Review targets and choose physical bindings before staging or saving."));
                        }
                        Ok(_) => model.as_mut().set_fbneo_import_status(qstring("FBNeo inspection cancelled. No draft was accepted.")),
                    Err(error) => model.as_mut().set_fbneo_import_status(qstring(format!("FBNeo inspection stopped: {error:#}"))),
                }
            });
        });
        if let Err(error) = worker {
            self.as_mut().rust_mut().fbneo_inspection_cancel = None;
            self.as_mut().set_fbneo_inspection_active(false);
            self.as_mut().set_fbneo_import_busy(false);
            self.as_mut().set_fbneo_import_status(qstring(format!(
                "Could not start FBNeo inspection: {error}"
            )));
            return qstring(format!("Could not start FBNeo inspection: {error}"));
        }
        QString::default()
    }

    pub fn begin_fbneo_inspection_import(
        mut self: Pin<&mut Self>,
        request: QString,
        report: QString,
        context: QString,
    ) -> QString {
        if self.as_ref().rust().fbneo_import_busy {
            return qstring("An FBNeo report import is already running.");
        }
        let request = request.to_string();
        let report = report.to_string();
        let context = context.to_string();
        if request.len() > 1024 * 1024
            || report.len() > 8 * 1024 * 1024
            || context.len() > 64 * 1024
        {
            return qstring("FBNeo import input exceeds its size limit.");
        }
        self.as_mut().set_fbneo_import_busy(true);
        self.as_mut().set_fbneo_import_draft(QString::default());
        self.as_mut().set_fbneo_import_status(qstring("Checking the existing report and current input-file hashes. No native core is being started…"));
        let qt_thread = self.as_ref().qt_thread();
        let worker = std::thread::Builder::new().name("lunchbox-fbneo-report-import".into()).spawn(move || {
            let result = (|| -> anyhow::Result<String> {
                let request: lunchbox_controller_probe::content_inspection::Request = serde_json::from_str(&request)?;
                let context: crate::controller_fbneo::InspectionImportContext = serde_json::from_str(&context)?;
                let draft = crate::controller_fbneo::import_inspection(&request, report.as_bytes(), context)?;
                Ok(serde_json::to_string_pretty(&draft)?)
            })();
            let _ = qt_thread.queue(move |mut model| {
                model.as_mut().set_fbneo_import_busy(false);
                match result {
                    Ok(draft) => {
                        model.as_mut().set_fbneo_import_draft(qstring(draft));
                        model.as_mut().set_fbneo_import_status(qstring("Report imported as an unassigned draft. Choose physical bindings, then stage and save the setup."));
                    }
                    Err(error) => model.as_mut().set_fbneo_import_status(qstring(format!("FBNeo import failed: {error:#}"))),
                }
            });
        });
        if let Err(error) = worker {
            self.as_mut().set_fbneo_import_busy(false);
            self.as_mut()
                .set_fbneo_import_status(qstring(format!("Could not start FBNeo import: {error}")));
            return qstring(format!("Could not start FBNeo import: {error}"));
        }
        QString::default()
    }

    pub fn fbneo_assignment_review_json(&self, configuration: QString) -> QString {
        let result = (|| -> anyhow::Result<serde_json::Value> {
            use crate::controller_fbneo::{BindingPart, InputEncoding, NativeLaunchSettings};
            let text = configuration.to_string();
            anyhow::ensure!(
                text.len() <= 8 * 1024 * 1024,
                "FBNeo review exceeds its size limit"
            );
            let setup: NativeLaunchSettings = serde_json::from_str(&text)?;
            // A draft may have unassigned targets. Preview validation is not
            // save/launch acceptance, so don't require complete assignments.
            anyhow::ensure!(
                setup.players.len() <= crate::controller_fbneo::FRONTEND_PORTS,
                "Too many FBNeo review players"
            );
            let mut players = std::collections::BTreeMap::new();
            for player in &setup.players {
                anyhow::ensure!(
                    player.assignments.len() <= 8192
                        && players.insert(player.port, player).is_none(),
                    "Duplicate or oversized FBNeo review player"
                );
            }
            let targets = crate::controller_fbneo::mapping_targets_from_records(
                &setup.expected_descriptors,
                &setup.expected_queries,
            )?;
            let observed: std::collections::BTreeSet<_> = targets
                .iter()
                .map(|target| target.address.clone())
                .collect();
            let mut rows = Vec::new();
            let mut calibration_reviews = Vec::new();
            let mut stale_assignments = Vec::new();
            for player in players.values() {
                for (index, assignment) in player.assignments.iter().enumerate() {
                    let reason = if assignment.target.port != player.port {
                        Some("Assignment belongs to a different native player port")
                    } else if !observed.contains(&assignment.target) {
                        Some("Native address is absent from the inspected contract")
                    } else if !crate::controller_fbneo::binding_parts(&assignment.target)
                        .contains(&assignment.part)
                        && !(assignment.target.device == 3
                            && assignment.part == BindingPart::Digital
                            && setup
                                .keyboard_bindings
                                .iter()
                                .any(|key| key.target == assignment.target))
                    {
                        Some("This native address does not support the saved binding part")
                    } else {
                        None
                    };
                    if let Some(reason) = reason {
                        stale_assignments.push(serde_json::json!({"player_port":player.port,
                            "index":index,"assignment":assignment,"reason":reason}));
                    }
                }
            }
            let relative_reviews = crate::controller_fbneo::relative_port_reviews(
                &targets,
                &self.rust().controller_mapping.relative_devices,
            );
            let (prepared_relative_sources, relative_source_error) =
                match setup.prepared_relative_sources() {
                    Ok(sources) => (Some(sources), String::new()),
                    Err(error) => (None, format!("{error:#}")),
                };
            let relative_only_ports = setup.relative_only_ports().unwrap_or_default();
            let keyboard_passthrough = setup.prepared_keyboard_passthrough()?;
            let absolute_sources = setup.prepared_absolute_sources()?;
            let gamepad_free_ports = setup.gamepad_free_ports().unwrap_or_default();
            let relative_platform_pending =
                !cfg!(all(target_os = "linux", target_pointer_width = "64"));
            // Invalid source preparation has already produced an explicit
            // relative_source_error. Keep every target in the pad review then.
            let gamepad_targets = setup.gamepad_targets().ok();
            // Measurement eligibility depends on controller/part/source, not
            // target address. Keep caches scoped to this immutable review only;
            // native completeness and alias checks remain target-specific.
            let mut eligibility: std::collections::BTreeMap<
                (String, BindingPart, String),
                Option<String>,
            > = std::collections::BTreeMap::new();
            #[cfg(all(target_os = "linux", target_pointer_width = "64"))]
            let mut axis_pair_cache: std::collections::BTreeMap<
                String,
                Vec<serde_json::Value>,
            > = std::collections::BTreeMap::new();
            let mut unavailable_calibrations = std::collections::BTreeMap::new();
            for player in players.values() {
                if absolute_sources.contains_key(&player.port)
                    && gamepad_free_ports.contains(&player.port)
                    && player.controller_id.is_empty()
                    && player.assignments.is_empty()
                {
                    calibration_reviews.push(serde_json::json!({"port":player.port,"absolute_only":true,
                        "required_parts":0,"mapped_parts":0,"mapped_percent":null,
                        "missing":[],"external_targets":[],"launch_ready":false,
                        "notice":"Saved absolute aim owns this port. Launch must receive a fresh position from both axes; runtime behavior is unverified."}));
                    continue;
                }
                if keyboard_passthrough == Some(player.port)
                    && gamepad_free_ports.contains(&player.port)
                    && player.controller_id.is_empty()
                    && player.assignments.is_empty()
                {
                    calibration_reviews.push(serde_json::json!({"port":player.port,
                        "keyboard_passthrough":true,"required_parts":0,"mapped_parts":0,
                        "mapped_percent":null,"missing":[],"external_targets":[],
                        "notice":"Native frontend keyboard input; not a selected or calibrated physical keyboard. No gamepad is required for this port. Runtime behavior is unverified.",
                        "launch_ready":false}));
                    continue;
                }
                if relative_only_ports.contains(&player.port)
                    && player.controller_id.is_empty()
                    && player.assignments.is_empty()
                {
                    calibration_reviews.push(serde_json::json!({"port":player.port,
                        "relative_only":true,"required_parts":0,"mapped_parts":0,
                        "mapped_percent":null,"missing":[],"external_targets":[],
                        "notice":"Prepared relative source covers this port; launch requires the opt-in frontend routing extension on 64-bit Linux. Runtime behavior is unverified.",
                        "launch_ready":false}));
                    continue;
                }
                let Some(calibration) = self
                    .rust()
                    .controller_mapping
                    .calibrations
                    .get(&player.controller_id)
                else {
                    unavailable_calibrations.insert(
                        player.port,
                        "Selected controller has no saved calibration".to_owned(),
                    );
                    continue;
                };
                if let Err(error) = calibration.validate() {
                    unavailable_calibrations
                        .insert(player.port, format!("Invalid saved calibration: {error:#}"));
                    continue;
                }
                #[cfg(all(target_os = "linux", target_pointer_width = "64"))]
                let review = (|| -> anyhow::Result<serde_json::Value> {
                    if absolute_sources.contains_key(&player.port) {
                        crate::controller_launch::fbneo_absolute_output_axes(calibration, &setup.physical_assignments(player)?)?;
                    }
                    if setup.keyboard_bindings.iter().any(|key| key.target.port == player.port) {
                        let devices = setup.players.iter().map(|player| (player.port, player.device)).collect();
                        let native_targets: Vec<_> = setup.keyboard_input_targets()?.into_iter().filter(|target| {
                            !(target.address.device == 2 && prepared_relative_sources.as_ref().is_some_and(|sources|
                                sources.contains_key(&((target.address.port + 1) as u8))))
                        }).collect();
                        crate::controller_launch::review_fbneo_keyboard_assignments(
                            &native_targets, player.port, calibration,
                            &setup.keyboard_bindings, &player.assignments, &devices,
                        )
                    } else {
                        crate::controller_launch::review_fbneo_assignments(
                            gamepad_targets.as_deref().unwrap_or(&targets), player.port, calibration, &player.assignments,
                        )
                    }
                })()
                .unwrap_or_else(|error| serde_json::json!({"port":player.port,"error":format!("{error:#}"),"launch_ready":false}));
                #[cfg(not(all(target_os = "linux", target_pointer_width = "64")))]
                let review = {
                    let _ = calibration;
                    serde_json::json!({"port":player.port,"error":"FBNeo normalized transport requires 64-bit Linux","launch_ready":false})
                };
                calibration_reviews.push(review);
            }
            for (port, error) in &unavailable_calibrations {
                calibration_reviews
                    .push(serde_json::json!({"port":port,"error":error,"launch_ready":false}));
            }
            for target in targets {
                if crate::controller_fbneo::arcade_aim_address(&target.address)
                    && absolute_sources.contains_key(&target.address.port)
                {
                    rows.push(serde_json::json!({"absolute_source":absolute_sources.get(&target.address.port),
                        "target":target,"controller_id":"","source_layout":null,"parts":[],"axis_pairs":[],
                        "requires_external_adapter":false,"runtime_verified":false}));
                    continue;
                }
                let passthrough_supported = target.address.device == 3
                    && crate::controller_fbneo::keyboard::udev_passthrough_key(target.address.id);
                if passthrough_supported
                    && keyboard_passthrough == Some(target.address.port)
                    && !setup
                        .keyboard_bindings
                        .iter()
                        .any(|key| key.target == target.address)
                {
                    rows.push(serde_json::json!({"target":target,"controller_id":"",
                        "source_layout":null,"parts":[],"axis_pairs":[],
                        "keyboard_passthrough":true,"keyboard_channel":null,
                        "keyboard_passthrough_supported":true,
                        "requires_external_adapter":false,"runtime_verified":false}));
                    continue;
                }
                let keyboard_channel = setup
                    .keyboard_bindings
                    .iter()
                    .find(|key| key.target == target.address)
                    .map(|key| key.channel);
                // Button channels can use the normal source picker. Analog
                // channel pairs remain explicit advanced assignments.
                let editable_parts: &[BindingPart] = if target.address.device == 3
                    && keyboard_channel.is_some_and(|channel| channel < 16)
                {
                    &[BindingPart::Digital]
                } else {
                    crate::controller_fbneo::binding_parts(&target.address)
                };
                let player = players
                    .get(&target.address.port)
                    .ok_or_else(|| anyhow::anyhow!("Reviewed input port has no selected player"))?;
                if target.address.device == 2
                    && prepared_relative_sources.as_ref().is_some_and(|sources| {
                        sources.contains_key(&((target.address.port + 1) as u8))
                    })
                {
                    rows.push(serde_json::json!({"relative_requirement":crate::controller_fbneo::relative_requirement(&target.address),
                        "mouse_destination_control":crate::controller_fbneo::mouse_destination_control(&target.address),
                        "relative_source":prepared_relative_sources.as_ref()
                            .and_then(|sources| sources.get(&((target.address.port + 1) as u8))),
                        "target":target,"controller_id":"","source_layout":null,
                        "parts":[],"axis_pairs":[],"requires_external_adapter":false,
                        "relative_source_selected":true,"relative_launch_pending":relative_platform_pending,
                        "relative_runtime_verified":false,
                        "lightgun_start_alias":false,"arcade_coordinate_alias":false,"pressure_alias":false}));
                    continue;
                }
                if let Some(error) = unavailable_calibrations.get(&player.port) {
                    rows.push(serde_json::json!({"target":target,"controller_id":player.controller_id,
                        "keyboard_passthrough_supported":passthrough_supported,
                        "keyboard_channel":keyboard_channel,
                        "source_layout":null,"parts":[],"axis_pairs":[],"calibration_error":error,
                        "requires_external_adapter":crate::controller_fbneo::binding_parts(&target.address).is_empty(),
                        "lightgun_start_alias":false,"arcade_coordinate_alias":false,"pressure_alias":false}));
                    continue;
                }
                let calibration = self
                    .rust()
                    .controller_mapping
                    .calibrations
                    .get(&player.controller_id)
                    .ok_or_else(|| {
                        anyhow::anyhow!("Selected FBNeo controller has no saved calibration")
                    })?;
                calibration.validate()?;
                let layout = crate::controller_catalog::catalog()
                    .layout(&calibration.layout)
                    .ok_or_else(|| anyhow::anyhow!("Unknown physical controller layout"))?;
                let mut parts = Vec::new();
                for &part in editable_parts {
                    let mut assigned = player.assignments.iter().filter(|assignment| {
                        assignment.target == target.address && assignment.part == part
                    });
                    let source = assigned.next().map(|assignment| assignment.source.as_str());
                    // Keep malformed drafts repairable. Editing this exact
                    // address/part replaces every duplicate; staging remains strict.
                    let duplicate_count = assigned.count();
                    let alias = crate::controller_fbneo::retroarch_address_alias(&target.address)
                        .map(|address| (address, part))
                        .or_else(|| {
                            // Pressure supplies the same-ID digital fallback;
                            // a digital button cannot supply pressure in reverse.
                            (target.address.device == 1
                                && target.address.index == 0
                                && target.address.id <= 15
                                && part == BindingPart::Digital)
                                .then(|| {
                                    (
                                        crate::controller_fbneo::InputAddress {
                                            port: target.address.port,
                                            device: 5,
                                            index: 2,
                                            id: target.address.id,
                                        },
                                        BindingPart::Pressure,
                                    )
                                })
                        });
                    let inherited = alias.as_ref().and_then(|(address, alias_part)| {
                        if !observed.contains(address) {
                            return None;
                        }
                        player.assignments.iter().find(|assignment| {
                            assignment.target == *address && assignment.part == *alias_part
                        })
                    });
                    let effective_source =
                        source.or_else(|| inherited.map(|assignment| assignment.source.as_str()));
                    let alias_assignment_count = inherited.map_or(0, |first| {
                        player
                            .assignments
                            .iter()
                            .filter(|assignment| {
                                assignment.target == first.target && assignment.part == first.part
                            })
                            .count()
                    });
                    let mut choices = Vec::new();
                    let mut source_errors = std::collections::BTreeMap::new();
                    for control in &layout.controls {
                        let error = eligibility
                            .entry((player.controller_id.clone(), part, control.id.clone()))
                            .or_insert_with(|| {
                                #[cfg(all(target_os = "linux", target_pointer_width = "64"))]
                                let error = crate::controller_launch::normalize_fbneo_assignments(
                                    calibration,
                                    &[crate::controller_fbneo::SourceBinding {
                                        target: target.address.clone(),
                                        part,
                                        source: control.id.clone(),
                                    }],
                                )
                                .err()
                                .map(|error| format!("{error:#}"));
                                #[cfg(not(all(target_os = "linux", target_pointer_width = "64")))]
                                let error =
                                    Some("FBNeo calibrated input requires 64-bit Linux".to_owned());
                                error
                            })
                            .clone();
                        if let Some(error) = error {
                            source_errors.insert(control.id.clone(), error);
                            continue;
                        }
                        if let Some(input) = calibration.bindings.get(&control.id) {
                            choices.push(serde_json::json!({"id":control.id,"label":control.label,
                                "native":input.native,"measured":input.axis.is_some()}));
                        }
                    }
                    let source_error = effective_source
                        .and_then(|source| source_errors.get(source))
                        .cloned()
                        .or_else(|| {
                            effective_source
                                .filter(|source| {
                                    !layout.controls.iter().any(|control| control.id == *source)
                                })
                                .map(|_| {
                                    "Source control is absent from the current layout".to_owned()
                                })
                        });
                    parts.push(
                        serde_json::json!({"part": part, "source": source, "effective_source":effective_source,
                            "destination_control":crate::controller_fbneo::destination_control(&target.address, part),
                            "destination_layout":crate::controller_fbneo::destination_layout(&target.address, part),
                            "inherited_from":if source.is_none() { inherited.map(|assignment| &assignment.target) } else { None },
                            "alias_source":inherited.map(|assignment| assignment.source.as_str()), "source_error":source_error,
                            "duplicate_count":duplicate_count,
                            "alias_assignment_count":alias_assignment_count,
                            "alias_target":inherited.map(|assignment| &assignment.target),
                            "alias_part":inherited.map(|assignment| assignment.part),
                            "choices": choices}),
                    );
                }
                #[cfg(all(target_os = "linux", target_pointer_width = "64"))]
                let axis_pairs = if crate::controller_fbneo::binding_parts(&target.address)
                    .contains(&BindingPart::Negative)
                    && crate::controller_fbneo::binding_parts(&target.address)
                        .contains(&BindingPart::Positive)
                {
                    if !axis_pair_cache.contains_key(&player.controller_id) {
                        axis_pair_cache.insert(
                            player.controller_id.clone(),
                            crate::controller_launch::fbneo_axis_pair_choices(calibration)?,
                        );
                    }
                    axis_pair_cache[&player.controller_id].clone()
                } else {
                    Vec::new()
                };
                #[cfg(not(all(target_os = "linux", target_pointer_width = "64")))]
                let axis_pairs: Vec<serde_json::Value> = Vec::new();
                let relative_requirement =
                    crate::controller_fbneo::relative_requirement(&target.address);
                let relative_candidates: Vec<_> = relative_requirement
                    .into_iter()
                    .flat_map(|requirement| {
                        self.rust()
                            .controller_mapping
                            .relative_devices
                            .iter()
                            .filter(move |device| requirement.matches_saved_device(device))
                    })
                    .map(|device| {
                        serde_json::json!({"event_path":device.event_path,
                        "input_identity":device.input_identity})
                    })
                    .collect();
                rows.push(serde_json::json!({"target": target, "axis_pairs":axis_pairs,
                    "keyboard_passthrough_supported":passthrough_supported,
                    "keyboard_channel":keyboard_channel,
                    "relative_requirement":relative_requirement,"relative_candidates":relative_candidates,
                    "mouse_destination_control":crate::controller_fbneo::mouse_destination_control(&target.address),
                    "controller_id": player.controller_id, "source_layout": calibration.layout, "parts": parts,
                    "requires_external_adapter": editable_parts.is_empty(),
                    "lightgun_start_alias": crate::controller_fbneo::lightgun_start_alias(&target.address),
                    "arcade_coordinate_alias": target.address.device == 1029,
                    "pressure_alias": target.encoding == InputEncoding::AnalogButtonPressure}));
            }
            Ok(
                serde_json::json!({"targets": rows, "calibration_reviews":calibration_reviews,
                "stale_assignments":stale_assignments,
                "relative_reviews":relative_reviews,
                "relative_sources":setup.relative_sources,
                "keyboard_passthrough_port":keyboard_passthrough,
                "absolute_sources":absolute_sources,
                "relative_only_ports":relative_only_ports,
                "prepared_relative_sources":prepared_relative_sources,
                "relative_source_error":relative_source_error,
                "relative_launch_pending":!setup.relative_sources.is_empty()
                    && (relative_platform_pending || !relative_source_error.is_empty()),
                "relative_runtime_verified":false,
                "relative_launch_requirement":"64-bit Linux, exact saved devices and the opt-in frontend routing extension; fresh input identity, route and focus checks occur at launch. Runtime behavior is unverified.",
                "error": "", "launch_ready": false,
                "notice": "Choices are saved physical calibrations, not live device validation. Full validation occurs when staging and launching."}),
            )
        })();
        qstring(match result {
            Ok(value) => value.to_string(),
            Err(error) => serde_json::json!({"targets": [], "error": format!("{error:#}"), "launch_ready": false}).to_string(),
        })
    }

    pub fn fbneo_controller_setup_json(&self, key: QString) -> QString {
        let key = key.to_string();
        self.rust()
            .controller_mapping
            .fbneo_launches
            .iter()
            .find(|setup| setup.identity_key() == key)
            .map(|setup| {
                qstring(serde_json::to_string_pretty(setup).expect("FBNeo setup serializes"))
            })
            .unwrap_or_default()
    }

    pub fn save_fbneo_controller_setup(
        mut self: Pin<&mut Self>,
        configuration: QString,
    ) -> QString {
        let result = (|| -> anyhow::Result<_> {
            let text = configuration.to_string();
            anyhow::ensure!(
                text.len() <= 8 * 1024 * 1024,
                "FBNeo setup exceeds its size limit"
            );
            let setup: crate::controller_fbneo::NativeLaunchSettings = serde_json::from_str(&text)?;
            setup.validate()?;
            #[cfg(all(target_os = "linux", target_pointer_width = "64"))]
            {
                let targets = setup.gamepad_targets()?;
                let current = self.as_ref();
                let gamepad_free_ports = setup.gamepad_free_ports()?;
                for player in &setup.players {
                    if player.controller_id.is_empty() && gamepad_free_ports.contains(&player.port)
                    {
                        continue;
                    }
                    let calibration = current
                        .rust()
                        .controller_mapping
                        .calibrations
                        .get(&player.controller_id)
                        .ok_or_else(|| {
                            anyhow::anyhow!(
                                "FBNeo player {} has no saved physical calibration",
                                player.port + 1
                            )
                        })?;
                    crate::controller_launch::validate_saved_fbneo_assignments(
                        &targets,
                        player.port,
                        calibration,
                        &setup.physical_assignments(player)?,
                    )?;
                    if setup
                        .absolute_sources
                        .iter()
                        .any(|source| source.port == player.port)
                    {
                        crate::controller_launch::fbneo_absolute_output_axes(
                            calibration,
                            &setup.physical_assignments(player)?,
                        )?;
                    }
                }
            }
            #[cfg(not(all(target_os = "linux", target_pointer_width = "64")))]
            anyhow::bail!(
                "FBNeo calibrated launch setup requires the 64-bit Linux input adapter; retain the text draft for a supported host"
            );
            let key = setup.identity_key();
            let mut mapping = self.as_ref().rust().controller_mapping.clone();
            mapping
                .fbneo_launches
                .retain(|previous| previous.identity_key() != key);
            mapping.fbneo_launches.push(setup);
            mapping.validate()?;
            Ok(mapping)
        })();
        match result {
            Ok(mapping) => {
                self.as_mut().rust_mut().controller_mapping = mapping;
                self.as_mut().controller_settings_changed();
                self.as_mut().set_message(qstring("FBNeo setup staged. Save settings to keep it. No core or input device was started; the reviewed contract will be checked again at launch."));
                QString::default()
            }
            Err(error) => qstring(format!("Invalid FBNeo controller setup: {error:#}")),
        }
    }

    pub fn remove_fbneo_controller_setup(mut self: Pin<&mut Self>, key: QString) -> QString {
        let key = key.to_string();
        let mut mapping = self.as_ref().rust().controller_mapping.clone();
        let count = mapping.fbneo_launches.len();
        mapping
            .fbneo_launches
            .retain(|setup| setup.identity_key() != key);
        if mapping.fbneo_launches.len() == count {
            return qstring("That FBNeo setup no longer exists. Refresh the list.");
        }
        self.as_mut().rust_mut().controller_mapping = mapping;
        self.as_mut().controller_settings_changed();
        self.as_mut().set_message(qstring("FBNeo setup removed from staged settings. Save settings to keep this change. Game files and physical calibrations are unchanged."));
        QString::default()
    }

    pub fn choose_controller_launch_mode(
        mut self: Pin<&mut Self>,
        core: QString,
        platform: QString,
        profile: QString,
    ) -> QString {
        let core = core.to_string();
        let platform = platform.to_string();
        let profile = profile.to_string();
        if !profile.is_empty()
            && !crate::controller_catalog::catalog()
                .platform_profiles(&core, &platform)
                .iter()
                .any(|mode| mode.id == profile && mode.explicit_selection)
        {
            return qstring(
                "This explicit controller mode is not available for this core/platform.",
            );
        }
        let key = crate::controller_launch::selection_key(&core, &platform);
        if profile.is_empty() {
            self.as_mut()
                .rust_mut()
                .controller_mapping
                .launch_mode_selections
                .remove(&key);
        } else {
            self.as_mut()
                .rust_mut()
                .controller_mapping
                .launch_mode_selections
                .insert(key, profile);
        }
        self.as_mut().controller_settings_changed();
        qstring(
            "Mode staged. Save settings to persist it. This selects a target mode, not new physical button mappings.",
        )
    }

    pub fn controller_diagram(&self, layout: QString, active: QString) -> QString {
        let Some(layout) = crate::controller_catalog::catalog().layout(&layout.to_string()) else {
            return QString::default();
        };
        let svg = crate::controller_catalog::svg(layout, &active.to_string());
        let encoded: String = url::form_urlencoded::byte_serialize(svg.as_bytes()).collect();
        // form_urlencoded uses '+' for spaces; data URLs require percent encoding.
        qstring(format!(
            "data:image/svg+xml;charset=utf-8,{}",
            encoded.replace('+', "%20")
        ))
    }

    pub fn cancel_native_controller_capture(mut self: Pin<&mut Self>) {
        let pinned = self.as_mut();
        let mut state = pinned.rust_mut();
        if let Some(cancel) = state.native_capture_cancel.take() {
            cancel.store(true, std::sync::atomic::Ordering::Relaxed);
        }
        state.native_capture_generation = state.native_capture_generation.wrapping_add(1);
        state.native_capture_device = None;
        state.native_capture_runtime = None;
        state.native_capture_report = None;
        #[cfg(target_os = "linux")]
        {
            state.native_capture = None;
        }
        self.as_mut().set_native_capture_busy(false);
        self.as_mut().set_native_capture_ready(false);
        self.as_mut().set_native_capture_results(qstring("[]"));
        self.as_mut()
            .set_native_capture_status(qstring("Capture cancelled; nothing saved."));
    }

    pub fn begin_native_controller_capture(
        mut self: Pin<&mut Self>,
        device: QString,
        runtime_key: QString,
    ) -> QString {
        #[cfg(not(target_os = "linux"))]
        {
            let _ = (device, runtime_key);
            return qstring("Native SDL2 capture currently requires Linux.");
        }
        #[cfg(target_os = "linux")]
        {
            if *self.as_ref().native_capture_busy() {
                return qstring("Wait for the current sample or cancel it first.");
            }
            let key_text = runtime_key.to_string();
            if key_text.len() > 4096 {
                return qstring("Native capture runtime key is oversized.");
            }
            let key: [String; 2] = match serde_json::from_str(&key_text) {
                Ok(key) => key,
                Err(_) => {
                    return qstring("Choose a saved native runtime before capturing SDL2 inputs.");
                }
            };
            let pinned = self.as_ref();
            let mapping = &pinned.rust().controller_mapping;
            let mut matches = mapping
                .bizhawk_launch
                .iter()
                .chain(&mapping.bizhawk_launches)
                .filter(|native| native.emulator_id == key[0] && native.scope_id() == key[1]);
            let Some(native) = matches.next().cloned() else {
                return qstring(
                    "The selected native runtime is no longer saved. Reopen capture to refresh it.",
                );
            };
            if matches.next().is_some() {
                return qstring("The selected native runtime scope is ambiguous.");
            }
            if let Err(error) = native.validate() {
                return qstring(error.to_string());
            }
            let id = device.to_string();
            let selected: Vec<_> = self
                .as_ref()
                .rust()
                .controller_inventory
                .as_ref()
                .into_iter()
                .flat_map(|inventory| &inventory.controllers)
                .filter(|device| device.stable_id == id)
                .map(|device| device.device_path.clone())
                .collect();
            if selected.len() != 1 {
                return qstring("Reconnect and uniquely select the controller before capture.");
            }
            let path = selected[0].clone();
            self.as_mut().cancel_native_controller_capture();
            self.as_mut().rust_mut().native_capture_device = Some(id);
            self.as_mut().rust_mut().native_capture_runtime = Some(key);
            let generation = self.as_ref().rust().native_capture_generation;
            let cancel = Arc::new(AtomicBool::new(false));
            self.as_mut().rust_mut().native_capture_cancel = Some(cancel.clone());
            self.as_mut().set_native_capture_busy(true);
            self.as_mut()
                .set_native_capture_status(qstring("Sampling released controls…"));
            let qt_thread = self.as_ref().qt_thread();
            let worker = std::thread::Builder::new().name("lunchbox-sdl2-release".into()).spawn(move || {
                let result = native.controlled_environment().and_then(|environment|
                    crate::controller_bizhawk::NativeGestureCapture::begin(
                        &path, &native.probe_program, &native.sdl_library, native.working_directory(), &native.exe_directory, &environment, &cancel));
                let _ = qt_thread.queue(move |mut model| {
                    if model.as_ref().rust().native_capture_generation != generation { return; }
                    model.as_mut().set_native_capture_busy(false);
                    match result {
                        Ok(capture) => {
                            model.as_mut().rust_mut().native_capture = Some(capture);
                            model.as_mut().set_native_capture_ready(true);
                            model.as_mut().set_native_capture_status(qstring("Released sample captured. Hold the intended control, then capture pressed state."));
                        }
                        Err(error) => {
                            model.as_mut().rust_mut().native_capture_cancel = None;
                            model.as_mut().set_native_capture_status(qstring(format!("Released capture failed: {error:#}")));
                        }
                    }
                });
            });
            if let Err(error) = worker {
                self.as_mut().cancel_native_controller_capture();
                return qstring(format!("Could not start capture: {error}"));
            }
            QString::default()
        }
    }

    pub fn finish_native_controller_capture(mut self: Pin<&mut Self>) -> QString {
        #[cfg(not(target_os = "linux"))]
        {
            return qstring("Native SDL2 capture currently requires Linux.");
        }
        #[cfg(target_os = "linux")]
        {
            if *self.as_ref().native_capture_busy() {
                return qstring("Wait for the current sample to finish.");
            }
            let Some(capture) = self.as_mut().rust_mut().native_capture.take() else {
                return qstring("Capture released controls first.");
            };
            self.as_mut().set_native_capture_ready(false);
            let generation = self.as_ref().rust().native_capture_generation;
            let cancel = self
                .as_ref()
                .rust()
                .native_capture_cancel
                .clone()
                .unwrap_or_else(|| Arc::new(AtomicBool::new(false)));
            self.as_mut().set_native_capture_busy(true);
            self.as_mut()
                .set_native_capture_status(qstring("Sampling held control…"));
            let qt_thread = self.as_ref().qt_thread();
            let worker = std::thread::Builder::new().name("lunchbox-sdl2-pressed".into()).spawn(move || {
                let result = capture.finish(&cancel).and_then(|report| {
                    let json = serde_json::to_string(&report.changes)?;
                    Ok((report, json))
                });
                let _ = qt_thread.queue(move |mut model| {
                    if model.as_ref().rust().native_capture_generation != generation { return; }
                    model.as_mut().set_native_capture_busy(false);
                    model.as_mut().rust_mut().native_capture_cancel = None;
                    match result {
                        Ok((report, json)) => {
                            model.as_mut().rust_mut().native_capture_report = Some(report);
                            model.as_mut().set_native_capture_results(qstring(json));
                            model.as_mut().set_native_capture_status(qstring("Gesture captured. Logical changes are shown for selection; nothing has been saved."));
                        }
                        Err(error) => model.as_mut().set_native_capture_status(qstring(format!("Pressed capture failed: {error:#}. Repeat both samples."))),
                    }
                });
            });
            if let Err(error) = worker {
                self.as_mut().cancel_native_controller_capture();
                return qstring(format!("Could not start capture: {error}"));
            }
            QString::default()
        }
    }

    pub fn save_native_controller_gesture(
        mut self: Pin<&mut Self>,
        device: QString,
        control: QString,
        selected: i32,
    ) -> QString {
        let id = device.to_string();
        let result = (|| -> anyhow::Result<crate::settings::ControllerMappingSettings> {
            let pinned = self.as_ref();
            let state = pinned.rust();
            anyhow::ensure!(
                !state.native_capture_busy
                    && state.native_capture_device.as_deref() == Some(id.as_str()),
                "This capture does not belong to the selected controller"
            );
            anyhow::ensure!(
                state
                    .controller_inventory
                    .as_ref()
                    .into_iter()
                    .flat_map(|inventory| &inventory.controllers)
                    .filter(|device| device.stable_id == id)
                    .count()
                    == 1,
                "Reconnect and uniquely select the captured controller"
            );
            let physical = state
                .controller_mapping
                .calibrations
                .get(&id)
                .ok_or_else(|| {
                    anyhow::anyhow!("Choose a layout and save physical calibration first")
                })?;
            anyhow::ensure!(
                physical.bindings.contains_key(&control.to_string()),
                "The selected physical control has not been calibrated"
            );
            let report = state
                .native_capture_report
                .as_ref()
                .ok_or_else(|| anyhow::anyhow!("Capture a gesture first"))?;
            let gesture = usize::try_from(selected)
                .ok()
                .and_then(|index| report.changes.get(index))
                .ok_or_else(|| anyhow::anyhow!("Select a measured logical change"))?;
            let runtime = state
                .native_capture_runtime
                .as_ref()
                .ok_or_else(|| anyhow::anyhow!("Capture has no runtime scope"))?;
            let mut calibration = match state
                .controller_mapping
                .sdl2_runtime_calibrations
                .iter()
                .find(|entry| {
                    entry.emulator_id == runtime[0]
                        && entry.scope == runtime[1]
                        && entry.controller_id == id
                })
                .map(|entry| &entry.calibration)
            {
                Some(saved) => {
                    anyhow::ensure!(
                        saved.layout == physical.layout && saved.context == report.context,
                        "Existing logical bindings use a different layout or runtime mapping; clear them before recapturing"
                    );
                    saved.clone()
                }
                None => crate::controller_bizhawk::LogicalCalibration {
                    layout: physical.layout.clone(),
                    context: report.context.clone(),
                    bindings: std::collections::BTreeMap::new(),
                },
            };
            calibration
                .bindings
                .insert(control.to_string(), gesture.clone());
            calibration.validate()?;
            let mut mapping = state.controller_mapping.clone();
            mapping.sdl2_runtime_calibrations.retain(|entry| {
                !(entry.emulator_id == runtime[0]
                    && entry.scope == runtime[1]
                    && entry.controller_id == id)
            });
            mapping.sdl2_runtime_calibrations.push(
                crate::controller_bizhawk::RuntimeLogicalCalibration {
                    emulator_id: runtime[0].clone(),
                    scope: runtime[1].clone(),
                    controller_id: id.clone(),
                    calibration,
                },
            );
            mapping.validate()?;
            Ok(mapping)
        })();
        match result {
            Ok(mapping) => {
                self.as_mut().rust_mut().controller_mapping = mapping;
                self.as_mut().controller_settings_changed();
                self.as_mut().set_native_capture_status(qstring(
                    "Runtime-scoped logical binding recorded. Other runtime and legacy bindings are unchanged. Save settings to keep it.",
                ));
                QString::default()
            }
            Err(error) => qstring(format!("Cannot save logical binding: {error:#}")),
        }
    }

    pub fn native_controller_player_layout_json(
        &self,
        player: QString,
        core: QString,
        emulator: QString,
    ) -> QString {
        let result = (|| -> anyhow::Result<serde_json::Value> {
            use anyhow::Context;
            let core = core.to_string();
            anyhow::ensure!(
                matches!(
                    core.as_str(),
                    "nymashock"
                        | "snes9x"
                        | "neshawk"
                        | "neshawk-snespad"
                        | "neshawk-powerpad"
                        | "smshawk-sms"
                        | "smshawk-gg"
                        | "smshawk-sg"
                        | "pcehawk"
                        | "turbonyma"
                        | "gpgx-three"
                        | "gpgx-six"
                        | "gpgx-activator"
                ),
                "Unknown native preview mode"
            );
            let text = player.to_string();
            anyhow::ensure!(text.len() <= 16384, "Native player preview is oversized");
            let player: crate::controller_bizhawk::NativePlayerSettings =
                serde_json::from_str(&text)?;
            if core != "nymashock" {
                anyhow::ensure!(
                    !player.dualshock
                        && !player.dualanalog
                        && !player.analog_joystick
                        && player.rhythm.is_none()
                        && !player.negcon
                        && player.pointer.is_none()
                        && !player.desktop_cursor
                        && player.analog_toggle_id.is_none()
                        && !player.rumble,
                    "Native digital preview requires controls without PlayStation modes or rumble"
                );
            }
            anyhow::ensure!(
                [
                    player.dualshock,
                    player.dualanalog,
                    player.analog_joystick,
                    player.rhythm.is_some(),
                    player.negcon,
                    player.pointer.is_some()
                ]
                .into_iter()
                .filter(|mode| *mode)
                .count()
                    <= 1,
                "Choose one native controller mode"
            );
            let calibration = self
                .rust()
                .controller_mapping
                .calibrations
                .get(&player.controller_id)
                .context("Save physical calibration before previewing this player")?;
            calibration.validate()?;
            let target = if core == "snes9x" || core == "neshawk-snespad" {
                "snes"
            } else if core == "neshawk" {
                "nes"
            } else if core == "neshawk-powerpad" {
                "nes-power-pad"
            } else if core == "smshawk-sms" {
                "master-system"
            } else if core == "smshawk-gg" {
                "gamegear"
            } else if core == "smshawk-sg" {
                "sg1000"
            } else if core == "pcehawk" {
                "pce-2"
            } else if core == "turbonyma" {
                "pce-turbonyma"
            } else if core == "gpgx-three" {
                "genesis-3"
            } else if core == "gpgx-six" {
                "genesis-6"
            } else if core == "gpgx-activator" {
                "genesis-activator"
            } else {
                crate::controller_bizhawk::mode_layout_id(
                    player.dualshock,
                    player.dualanalog,
                    player.analog_joystick,
                    player.rhythm,
                    player.negcon,
                    player.pointer,
                )
            };
            let catalog = crate::controller_catalog::catalog();
            let source = catalog
                .layout(&calibration.layout)
                .context("Unknown physical layout")?;
            let destination = catalog
                .layout(target)
                .context("Native target layout is missing")?;
            anyhow::ensure!(
                player.dualshock == player.analog_toggle_id.is_some(),
                "Assign a mode-toggle control only for DualShock"
            );
            if let Some(toggle) = &player.analog_toggle_id {
                anyhow::ensure!(
                    calibration.bindings.contains_key(toggle),
                    "The mode toggle has no physical calibration"
                );
            }
            anyhow::ensure!(
                !player.desktop_cursor
                    || matches!(
                        player.pointer,
                        Some(
                            crate::controller_bizhawk::PointerPeripheral::GunCon
                                | crate::controller_bizhawk::PointerPeripheral::Justifier
                        )
                    ),
                "Desktop cursor is only available for native lightgun modes"
            );
            let available = crate::controller_bizhawk::calibrated_control_ids(
                calibration,
                None,
                player.analog_toggle_id.as_deref(),
            );
            let requested = crate::controller_bizhawk::requested_control_ids(
                destination,
                player.desktop_cursor,
            );
            let resolution = crate::controller_bizhawk::guided::resolve(
                calibration,
                source,
                destination,
                &available,
                &requested,
            )?;
            let make_rows = |resolution: &crate::controller_layout::Resolution,
                             description: &str|
             -> Vec<serde_json::Value> {
                destination.controls.iter()
                .filter(|control| !player.desktop_cursor || !control.analog)
                .map(|control| {
                    let physical = resolution.assignments.get(&control.id)
                        .and_then(|id| source.controls.iter().find(|candidate| candidate.id == *id));
                    serde_json::json!({"target_id":control.id,"target":control.label,
                        "physical_id":physical.map(|control| &control.id),
                        "physical":physical.map(|control| control.label.as_str()).unwrap_or("Unmapped"),
                        "reason":resolution.rules.get(&control.id).map(|rule| rule.description())
                            .or_else(|| resolution.missing.get(&control.id).map(|missing| missing.description())).unwrap_or("No assignment"),
                        "output":description})
                }).collect()
            };
            let rows = make_rows(
                &resolution,
                "Physical calibration candidate; native translation not checked",
            );
            let logical_result = (|| -> anyhow::Result<Vec<serde_json::Value>> {
                anyhow::ensure!(
                    !player.normalized_input,
                    "Normalized sessions derive SDL bindings from the launch-owned virtual device; saved physical SDL bindings do not apply"
                );
                let emulator = emulator.to_string();
                anyhow::ensure!(
                    !emulator.trim().is_empty() && emulator.len() <= 1024,
                    "Choose the exact emulator installation before previewing saved SDL bindings"
                );
                let scope = match core.as_str() {
                    "neshawk-snespad" | "neshawk-powerpad" => "neshawk",
                    "smshawk-sms" => "master-system",
                    "smshawk-gg" => "gamegear",
                    "smshawk-sg" => "sg1000",
                    "gpgx-three" | "gpgx-six" | "gpgx-activator" => "gpgx",
                    other => other,
                };
                let (logical, scoped) = self
                    .rust()
                    .controller_mapping
                    .logical_calibration_for(&emulator, scope, &player.controller_id)
                    .context("No saved logical SDL calibration for this controller")?;
                logical.validate()?;
                anyhow::ensure!(
                    logical.layout == calibration.layout,
                    "Physical and SDL calibration layouts disagree"
                );
                if core == "neshawk-powerpad" {
                    crate::controller_bizhawk::neshawk::validate_power_inputs(
                        calibration,
                        Some(logical),
                    )?;
                }
                if core == "gpgx-activator" {
                    crate::controller_bizhawk::gpgx::validate_activator_inputs(
                        calibration,
                        Some(logical),
                    )?;
                }
                let available = crate::controller_bizhawk::calibrated_control_ids(
                    calibration,
                    Some(logical),
                    player.analog_toggle_id.as_deref(),
                );
                let resolution = crate::controller_bizhawk::guided::resolve(
                    calibration,
                    source,
                    destination,
                    &available,
                    &requested,
                )?;
                Ok(make_rows(
                    &resolution,
                    if scoped {
                        "Runtime-scoped SDL candidate; native translation not checked"
                    } else {
                        "Legacy fallback SDL candidate; native translation not checked"
                    },
                ))
            })();
            let (logical_rows, logical_error) = match logical_result {
                Ok(rows) => (rows, String::new()),
                Err(error) => (Vec::new(), format!("{error:#}")),
            };
            let mut exclusions = Vec::new();
            if core == "gpgx-activator" {
                if let Err(error) =
                    crate::controller_bizhawk::gpgx::validate_activator_inputs(calibration, None)
                {
                    exclusions.push(format!(
                        "Activator mapping lacks independent sensor inputs: {error:#}"
                    ));
                }
            }
            if core == "neshawk-powerpad" {
                if let Err(error) =
                    crate::controller_bizhawk::neshawk::validate_power_inputs(calibration, None)
                {
                    exclusions.push(format!(
                        "Power Pad mapping is incomplete or not independently pressable: {error:#}"
                    ));
                }
            }
            if let Some(toggle) = &player.analog_toggle_id {
                let label = source
                    .controls
                    .iter()
                    .find(|control| control.id == *toggle)
                    .map(|control| control.label.as_str())
                    .unwrap_or(toggle);
                exclusions.push(format!("Source {label} [{toggle}] is reserved for the DualShock analog-mode toggle, not gameplay. Its native toggle translation still needs validation."));
            }
            if player.desktop_cursor {
                for control in destination.controls.iter().filter(|control| control.analog) {
                    exclusions.push(format!("Destination {} [{}] is assigned to the explicitly selected desktop-cursor path and excluded from controller mapping. Screen coordinates are not verified here.", control.label, control.id));
                }
            }
            Ok(
                serde_json::json!({"source_layout":calibration.layout,"target_layout":target,
                "rows":rows,"logical_rows":logical_rows,"logical_error":logical_error,
                "exclusions":exclusions,"error":"","launch_ready":false}),
            )
        })();
        qstring(match result {
            Ok(value) => value.to_string(),
            Err(error) => {
                serde_json::json!({"error":format!("{error:#}"),"launch_ready":false}).to_string()
            }
        })
    }

    pub fn native_controller_runtime_json(&self) -> QString {
        qstring(
            serde_json::to_string(&self.rust().controller_mapping.bizhawk_launch)
                .expect("native runtime settings serialize"),
        )
    }

    pub fn native_controller_runtimes_json(&self) -> QString {
        let mapping = &self.rust().controller_mapping;
        let setups: Vec<_> = mapping
            .bizhawk_launch
            .iter()
            .chain(&mapping.bizhawk_launches)
            .map(|setup| {
                serde_json::json!({
                    "key": [setup.emulator_id.as_str(), setup.scope_id()],
                    "name": format!("{} · {}", setup.scope_id(), setup.emulator_id),
                    "configuration": setup,
                })
            })
            .collect();
        qstring(serde_json::to_string(&setups).expect("native setups serialize"))
    }

    pub fn native_controller_emulators_json(&self) -> QString {
        let result = (|| -> anyhow::Result<Vec<serde_json::Value>> {
            let path = crate::catalog::requested_database_path()
                .ok_or_else(|| anyhow::anyhow!("No emulator database found"))?;
            let connection =
                crate::catalog::open_read_only(&path, "Native controller runtime choices")?;
            let mut statement = connection.prepare(
                "SELECT id, name FROM emulators WHERE lower(name) = 'bizhawk' ORDER BY id LIMIT 64",
            )?;
            let rows = statement
                .query_map([], |row| {
                    Ok(serde_json::json!({
                        "id": row.get::<_, String>(0)?, "name": row.get::<_, String>(1)?
                    }))
                })?
                .collect::<rusqlite::Result<Vec<_>>>()?;
            Ok(rows)
        })();
        let value = match result {
            Ok(choices) => serde_json::json!({"choices": choices, "error": ""}),
            Err(error) => serde_json::json!({"choices": [], "error": format!("{error:#}")}),
        };
        qstring(value.to_string())
    }

    pub fn remove_native_controller_runtime(
        mut self: Pin<&mut Self>,
        expected: QString,
    ) -> QString {
        let result = (|| -> anyhow::Result<_> {
            let text = expected.to_string();
            anyhow::ensure!(text.len() <= 64 * 1024, "Native setup is oversized");
            let expected: crate::controller_bizhawk::NativeLaunchSettings =
                serde_json::from_str(&text)?;
            expected.validate()?;
            let mut mapping = self.as_ref().rust().controller_mapping.clone();
            let matches: Vec<_> = mapping
                .bizhawk_launch
                .iter()
                .chain(&mapping.bizhawk_launches)
                .filter(|saved| {
                    saved.emulator_id == expected.emulator_id
                        && saved.scope_id() == expected.scope_id()
                })
                .collect();
            anyhow::ensure!(
                matches.len() == 1 && *matches[0] == expected,
                "The selected setup changed or disappeared. Reload it before removing it."
            );
            if mapping.bizhawk_launch.as_ref() == Some(&expected) {
                mapping.bizhawk_launch = None;
            } else {
                mapping.bizhawk_launches.retain(|saved| saved != &expected);
            }
            if mapping.bizhawk_launch.is_none() && !mapping.bizhawk_launches.is_empty() {
                mapping.bizhawk_launch = Some(mapping.bizhawk_launches.remove(0));
            }
            mapping.validate()?;
            Ok(mapping)
        })();
        match result {
            Ok(mapping) => {
                self.as_mut().cancel_native_controller_capture();
                self.as_mut().rust_mut().controller_mapping = mapping;
                self.as_mut().controller_settings_changed();
                self.as_mut().set_message(qstring("Selected native setup removed from staged settings. All calibration records and other setups are preserved. Save settings to keep the change."));
                QString::default()
            }
            Err(error) => qstring(format!("Cannot remove native setup: {error:#}")),
        }
    }

    pub fn save_native_controller_runtime(
        mut self: Pin<&mut Self>,
        configuration: QString,
        expected: QString,
    ) -> QString {
        let result = (|| -> anyhow::Result<_> {
            let text = configuration.to_string();
            anyhow::ensure!(
                text.len() <= 64 * 1024,
                "Native runtime configuration is oversized"
            );
            let runtime: Option<crate::controller_bizhawk::NativeLaunchSettings> =
                serde_json::from_str(&text)?;
            let expected_text = expected.to_string();
            anyhow::ensure!(
                expected_text.len() <= 64 * 1024,
                "Native setup baseline is oversized"
            );
            let expected: Option<crate::controller_bizhawk::NativeLaunchSettings> =
                serde_json::from_str(&expected_text)?;
            let mut mapping = self.as_ref().rust().controller_mapping.clone();
            if let Some(next) = &runtime {
                next.validate()?;
                let baseline = expected.as_ref().filter(|saved| {
                    saved.emulator_id == next.emulator_id && saved.scope_id() == next.scope_id()
                });
                let matching: Vec<_> = mapping
                    .bizhawk_launch
                    .iter()
                    .chain(&mapping.bizhawk_launches)
                    .filter(|saved| {
                        saved.emulator_id == next.emulator_id && saved.scope_id() == next.scope_id()
                    })
                    .collect();
                anyhow::ensure!(
                    matching.len() <= 1 && matching.first().copied() == baseline,
                    "This emulator/system setup already exists or changed since loading. Load its current saved version before recording changes."
                );
                mapping.bizhawk_launches.retain(|saved| {
                    saved.emulator_id != next.emulator_id || saved.scope_id() != next.scope_id()
                });
                if let Some(previous) = mapping.bizhawk_launch.take() {
                    if previous.emulator_id != next.emulator_id
                        || previous.scope_id() != next.scope_id()
                    {
                        mapping.bizhawk_launches.push(previous);
                    }
                }
            } else {
                mapping.bizhawk_launches.clear();
            }
            mapping.bizhawk_launch = runtime;
            mapping.validate()?;
            Ok(mapping)
        })();
        match result {
            Ok(mapping) => {
                self.as_mut().cancel_native_controller_capture();
                self.as_mut().rust_mut().controller_mapping = mapping;
                self.as_mut().controller_settings_changed();
                self.as_mut().set_message(qstring("Native controller runtime setup recorded. Save settings to keep it. No emulator or input probe was started."));
                QString::default()
            }
            Err(error) => qstring(format!("Invalid native runtime setup: {error:#}")),
        }
    }

    pub fn scoped_native_controller_calibration_json(
        &self,
        device: QString,
        runtime_key: QString,
    ) -> QString {
        let key: [String; 2] = match serde_json::from_str(&runtime_key.to_string()) {
            Ok(key) => key,
            Err(_) => return qstring("{}"),
        };
        let id = device.to_string();
        self.rust()
            .controller_mapping
            .sdl2_runtime_calibrations
            .iter()
            .find(|entry| {
                entry.emulator_id == key[0] && entry.scope == key[1] && entry.controller_id == id
            })
            .map(|entry| {
                qstring(
                    serde_json::to_string(&entry.calibration)
                        .expect("scoped calibration serializes"),
                )
            })
            .unwrap_or_else(|| qstring("{}"))
    }

    pub fn clear_scoped_native_controller_calibration(
        mut self: Pin<&mut Self>,
        device: QString,
        runtime_key: QString,
    ) {
        let key: [String; 2] = match serde_json::from_str(&runtime_key.to_string()) {
            Ok(key) => key,
            Err(_) => return,
        };
        let id = device.to_string();
        self.as_mut().cancel_native_controller_capture();
        self.as_mut()
            .rust_mut()
            .controller_mapping
            .sdl2_runtime_calibrations
            .retain(|entry| {
                !(entry.emulator_id == key[0] && entry.scope == key[1] && entry.controller_id == id)
            });
        self.as_mut().controller_settings_changed();
        self.as_mut().set_native_capture_status(qstring("Selected runtime bindings cleared. Legacy fallback and other runtime bindings remain unchanged."));
    }

    pub fn native_controller_calibration_json(&self, device: QString) -> QString {
        self.rust()
            .controller_mapping
            .sdl2_calibrations
            .get(&device.to_string())
            .map(|saved| {
                qstring(serde_json::to_string(saved).expect("logical calibration serializes"))
            })
            .unwrap_or_else(|| qstring("{}"))
    }

    pub fn clear_native_controller_calibration(mut self: Pin<&mut Self>, device: QString) {
        let removed = self
            .as_mut()
            .rust_mut()
            .controller_mapping
            .sdl2_calibrations
            .remove(&device.to_string());
        if removed.is_some() {
            self.as_mut().controller_settings_changed();
            self.as_mut().set_native_capture_status(qstring("Logical bindings cleared from staged settings. Save settings to keep this change. Physical calibration is unchanged."));
        }
    }

    pub fn saved_controller_choices_json(&self) -> QString {
        let choices: Vec<_> = self
            .rust()
            .controller_mapping
            .calibrations
            .iter()
            .map(|(id, calibration)| {
                serde_json::json!({
                    "id": id, "layout": calibration.layout,
                    "name": self.rust().controller_mapping.device_names.get(id).unwrap_or(id),
                    "valid": calibration.validate().is_ok()
                })
            })
            .collect();
        qstring(serde_json::to_string(&choices).expect("saved controller choices serialize"))
    }

    pub fn controller_calibration_json(&self, device: QString) -> QString {
        self.rust()
            .controller_mapping
            .calibrations
            .get(&device.to_string())
            .map(|calibration| {
                qstring(serde_json::to_string(calibration).expect("calibration serializes"))
            })
            .unwrap_or_else(|| qstring("{}"))
    }

    /// Validate one completed recording without touching inventory or settings.
    /// Whole-calibration identity/collision validation remains at the save gate.
    pub fn validate_controller_capture(
        &self,
        layout: QString,
        control: QString,
        binding: QString,
    ) -> QString {
        let result = (|| -> anyhow::Result<()> {
            let layout = layout.to_string();
            let control = control.to_string();
            let binding = binding.to_string();
            anyhow::ensure!(
                layout.len() <= 1024 && control.len() <= 1024 && binding.len() <= 65536,
                "Completed controller recording is oversized"
            );
            let input: crate::controller_catalog::InputBinding = serde_json::from_str(&binding)?;
            let backend = if crate::controller_sdl3::is_binding(&input) {
                crate::controller_sdl3::BACKEND
            } else {
                "gilrs-0.11"
            };
            crate::controller_catalog::Calibration {
                target_mappings: Default::default(),
                layout,
                os: std::env::consts::OS.into(),
                backend: backend.into(),
                bindings: std::collections::BTreeMap::from([(control, input)]),
            }
            .validate()
        })();
        match result {
            Ok(()) => qstring(""),
            Err(error) => qstring(format!("Invalid completed recording: {error:#}")),
        }
    }

    pub fn save_controller_calibration_if_unchanged(
        self: Pin<&mut Self>,
        device: QString,
        layout: QString,
        bindings: QString,
        expected: QString,
    ) -> QString {
        // Compare the exact serialized baseline returned to the wizard. Both
        // comparison and the existing write execute synchronously on this
        // settings model; unrelated controller changes do not invalidate it.
        if self
            .as_ref()
            .controller_calibration_json(device.clone())
            .to_string()
            != expected.to_string()
        {
            return qstring(
                "This controller's saved calibration changed while the wizard was open. Nothing was overwritten. Cancel and reopen calibration to load the current bindings.",
            );
        }
        self.save_controller_calibration(device, layout, bindings)
    }

    pub fn save_controller_calibration(
        mut self: Pin<&mut Self>,
        device: QString,
        layout: QString,
        bindings: QString,
    ) -> QString {
        let id = device.to_string();
        if !self
            .as_ref()
            .rust()
            .controller_inventory
            .as_ref()
            .is_some_and(|inventory| {
                inventory
                    .controllers
                    .iter()
                    .any(|device| device.stable_id == id)
            })
        {
            return qstring("Reconnect this controller before saving its calibration.");
        }
        let bindings = match serde_json::from_str(&bindings.to_string()) {
            Ok(bindings) => bindings,
            Err(error) => return qstring(format!("Invalid calibration: {error}")),
        };
        let mut calibration = crate::controller_catalog::Calibration {
            target_mappings: Default::default(),
            layout: layout.to_string(),
            os: std::env::consts::OS.into(),
            backend: crate::controller_sdl3::backend(&bindings).into(),
            bindings,
        };
        if let Err(error) = calibration.validate() {
            return qstring(error.to_string());
        }
        // Preserve unaffected logical gestures, but never carry a measurement
        // onto a different physical control just because its semantic ID stayed.
        let previous = self
            .as_ref()
            .rust()
            .controller_mapping
            .calibrations
            .get(&id)
            .cloned();
        if let Some(old) = previous
            .as_ref()
            .filter(|old| old.layout == calibration.layout && old.os == calibration.os)
        {
            calibration.target_mappings = old
                .target_mappings
                .iter()
                .map(|(profile, choices)| {
                    let retained = choices
                        .iter()
                        .filter(|(_, source)| {
                            old.bindings.get(*source) == calibration.bindings.get(*source)
                                && calibration.bindings.contains_key(*source)
                        })
                        .map(|(a, b)| (a.clone(), b.clone()))
                        .collect();
                    (profile.clone(), retained)
                })
                .collect();
        }
        let mut invalidated = 0;
        if let Some(logical) = self
            .as_mut()
            .rust_mut()
            .controller_mapping
            .sdl2_calibrations
            .get_mut(&id)
        {
            let before = logical.bindings.len();
            logical.bindings.retain(|control, _| {
                logical.layout == calibration.layout
                    && previous.as_ref().is_some_and(|old| {
                        old.os == calibration.os
                            && old.backend == calibration.backend
                            && old.bindings.get(control) == calibration.bindings.get(control)
                    })
                    && calibration.bindings.contains_key(control)
            });
            invalidated = before - logical.bindings.len();
        }
        if self
            .as_ref()
            .rust()
            .controller_mapping
            .sdl2_calibrations
            .get(&id)
            .is_some_and(|logical| logical.bindings.is_empty())
        {
            self.as_mut()
                .rust_mut()
                .controller_mapping
                .sdl2_calibrations
                .remove(&id);
        }
        for entry in &mut self
            .as_mut()
            .rust_mut()
            .controller_mapping
            .sdl2_runtime_calibrations
        {
            if entry.controller_id != id {
                continue;
            }
            let logical = &mut entry.calibration;
            let before = logical.bindings.len();
            logical.bindings.retain(|control, _| {
                logical.layout == calibration.layout
                    && previous.as_ref().is_some_and(|old| {
                        old.os == calibration.os
                            && old.backend == calibration.backend
                            && old.bindings.get(control) == calibration.bindings.get(control)
                    })
                    && calibration.bindings.contains_key(control)
            });
            invalidated += before - logical.bindings.len();
        }
        self.as_mut()
            .rust_mut()
            .controller_mapping
            .sdl2_runtime_calibrations
            .retain(|entry| !entry.calibration.bindings.is_empty());
        if self.as_ref().rust().native_capture_device.as_deref() == Some(id.as_str()) {
            self.as_mut().cancel_native_controller_capture();
        }
        self.as_mut()
            .rust_mut()
            .controller_mapping
            .calibrations
            .insert(id, calibration);
        self.as_mut().controller_settings_changed();
        self.as_mut().set_message(qstring(format!("Controller calibration recorded. Save settings to keep it. {invalidated} stale SDL2 logical bindings removed; unchanged bindings were preserved. Supported launch adapters apply calibration automatically; others still need adapter support.")));
        QString::default()
    }

    pub fn controller_mapping_preview(
        &self,
        layout: QString,
        bindings: QString,
        profile: QString,
    ) -> QString {
        let result = (|| -> anyhow::Result<_> {
            let bindings = serde_json::from_str(&bindings.to_string())?;
            let cal = crate::controller_catalog::Calibration {
                target_mappings: Default::default(),
                layout: layout.to_string(),
                os: std::env::consts::OS.into(),
                backend: crate::controller_sdl3::backend(&bindings).into(),
                bindings,
            };
            cal.plan(&profile.to_string())
        })();
        match result {
            Ok(plan) => qstring(serde_json::to_string(&plan).expect("plan serializes")),
            Err(error) => qstring(serde_json::json!({"rows":[],"warnings":[error.to_string()],"automatic_launch_ready":false}).to_string()),
        }
    }

    pub fn guided_controller_preview(
        &self,
        device: QString,
        profile: QString,
        choices: QString,
    ) -> QString {
        let result = (|| -> anyhow::Result<_> {
            use anyhow::Context;
            let mut calibration = self
                .rust()
                .controller_mapping
                .calibrations
                .get(&device.to_string())
                .context("Set up this controller first")?
                .clone();
            anyhow::ensure!(
                calibration.os == std::env::consts::OS,
                "Record this controller on this operating system first"
            );
            let profile = profile.to_string();
            calibration
                .target_mappings
                .insert(profile.clone(), serde_json::from_str(&choices.to_string())?);
            let plan = calibration.plan(&profile)?;
            Ok(
                serde_json::json!({"rows":plan.rows,"error":"", "launch_ready":plan.automatic_launch_ready}),
            )
        })();
        qstring(match result {
            Ok(value) => value.to_string(),
            Err(error) => {
                serde_json::json!({"rows":[],"error":error.to_string(),"launch_ready":false})
                    .to_string()
            }
        })
    }

    pub fn controller_player_order_json(&self) -> QString {
        qstring(
            serde_json::to_string(
                &self
                    .rust()
                    .controller_mapping
                    .player_mappings
                    .iter()
                    .map(|player| player.controller_id.clone().unwrap_or_default())
                    .collect::<Vec<_>>(),
            )
            .expect("player order serializes"),
        )
    }

    pub fn save_controller_player_order(mut self: Pin<&mut Self>, players: QString) -> QString {
        let result = (|| -> anyhow::Result<_> {
            anyhow::ensure!(!*self.as_ref().busy(), "Wait for settings to finish saving");
            let mut ids: Vec<String> = serde_json::from_str(&players.to_string())?;
            while ids.last().is_some_and(String::is_empty) {
                ids.pop();
            }
            anyhow::ensure!(ids.len() <= 16, "Too many players");
            let mut seen = std::collections::HashSet::new();
            for id in &ids {
                anyhow::ensure!(
                    !id.is_empty(),
                    "Assign earlier players before later players"
                );
                anyhow::ensure!(
                    seen.insert(id),
                    "A controller can only be assigned to one player"
                );
                anyhow::ensure!(
                    self.as_ref()
                        .rust()
                        .controller_inventory
                        .as_ref()
                        .is_some_and(|inventory| inventory
                            .controllers
                            .iter()
                            .any(|device| device.stable_id == *id))
                        || self
                            .as_ref()
                            .rust()
                            .controller_mapping
                            .player_mappings
                            .iter()
                            .any(|player| player.controller_id.as_deref() == Some(id)),
                    "Reconnect this controller before assigning it to a player"
                );
            }
            let model = self.as_ref();
            let mapping = &model.rust().controller_mapping;
            let players: Vec<_> = ids
                .into_iter()
                .map(|id| {
                    mapping
                        .player_mappings
                        .iter()
                        .find(|p| p.controller_id.as_deref() == Some(id.as_str()))
                        .cloned()
                        .unwrap_or(crate::settings::ControllerPlayerMapping {
                            controller_id: Some(id),
                            ..Default::default()
                        })
                })
                .collect();
            SettingsStore::open_default()?.save_controller_player_order(&players)?;
            Ok(players)
        })();
        match result {
            Ok(players) => {
                self.as_mut().rust_mut().controller_mapping.player_mappings = players;
                self.as_mut()
                    .rust_mut()
                    .controller_mapping
                    .explicit_player_selection = true;
                self.as_mut().bump_controller_revision();
                qstring("")
            }
            Err(error) => qstring(error.to_string()),
        }
    }

    pub fn persist_controller_calibration(mut self: Pin<&mut Self>, device: QString) -> QString {
        let result = (|| -> anyhow::Result<()> {
            use anyhow::Context;
            anyhow::ensure!(!*self.as_ref().busy(), "Wait for settings to finish saving");
            let calibration = self
                .as_ref()
                .rust()
                .controller_mapping
                .calibrations
                .get(&device.to_string())
                .context("Set up this controller first")?
                .clone();
            SettingsStore::open_default()?
                .save_controller_calibration(&device.to_string(), &calibration)
        })();
        match result {
            Ok(()) => {
                self.as_mut().bump_controller_revision();
                qstring("")
            }
            Err(error) => qstring(format!("Could not save controller: {error:#}")),
        }
    }

    pub fn controller_target_profile(&self, emulator: QString, platform: QString) -> QString {
        crate::controller_target::Scope::from_label(&emulator.to_string(), &platform.to_string())
            .ok()
            .and_then(|scope| {
                self.rust()
                    .controller_mapping
                    .guided_target_selections
                    .get(&scope.key())
            })
            .map(qstring)
            .unwrap_or_default()
    }

    pub fn save_controller_target_profile(
        mut self: Pin<&mut Self>,
        emulator: QString,
        platform: QString,
        profile: QString,
    ) -> QString {
        let result = (|| -> anyhow::Result<_> {
            anyhow::ensure!(!*self.as_ref().busy(), "Wait for settings to finish saving");
            let scope = crate::controller_target::Scope::from_label(
                &emulator.to_string(),
                &platform.to_string(),
            )?;
            let id = profile.to_string();
            SettingsStore::open_default()?.save_controller_target(&scope, &id)?;
            Ok((scope.key(), id))
        })();
        match result {
            Ok((key, id)) => {
                self.as_mut()
                    .rust_mut()
                    .controller_mapping
                    .guided_target_selections
                    .insert(key, id);
                self.as_mut().bump_controller_revision();
                qstring("")
            }
            Err(error) => qstring(format!("Could not save target controller: {error:#}")),
        }
    }

    pub fn save_guided_controller_mapping(
        mut self: Pin<&mut Self>,
        device: QString,
        profile: QString,
        choices: QString,
        expected: QString,
    ) -> QString {
        let result = (|| -> anyhow::Result<_> {
            use anyhow::{Context, ensure};
            ensure!(!*self.as_ref().busy(), "Wait for settings to finish saving");
            ensure!(
                self.as_ref()
                    .controller_calibration_json(device.clone())
                    .to_string()
                    == expected.to_string(),
                "Controller setup changed. Select it again before saving"
            );
            let mut calibration = self
                .as_ref()
                .rust()
                .controller_mapping
                .calibrations
                .get(&device.to_string())
                .context("Set up this controller first")?
                .clone();
            let profile_id = profile.to_string();
            ensure!(
                calibration.os == std::env::consts::OS,
                "Record this controller on this operating system first"
            );
            calibration.target_mappings.insert(
                profile_id.clone(),
                serde_json::from_str(&choices.to_string())?,
            );
            let plan = calibration.plan(&profile_id)?;
            let db = crate::controller_catalog::catalog();
            let profile = db
                .emulator_profiles
                .iter()
                .find(|p| p.id == profile_id)
                .context("Choose a target")?;
            let target = db
                .layout(&profile.target_layout)
                .context("Missing target layout")?;
            ensure!(
                plan.rows.iter().all(|row| row.input.is_some()
                    || target
                        .controls
                        .iter()
                        .any(|c| c.id == row.target_id && c.optional)),
                "Some required controls are missing. Record them or choose another controller"
            );
            SettingsStore::open_default()?
                .save_controller_calibration(&device.to_string(), &calibration)?;
            Ok(calibration)
        })();
        match result {
            Ok(calibration) => {
                self.as_mut()
                    .rust_mut()
                    .controller_mapping
                    .calibrations
                    .insert(device.to_string(), calibration);
                self.as_mut().bump_controller_revision();
                qstring("")
            }
            Err(error) => qstring(error.to_string()),
        }
    }

    pub fn controller_layout_mapping(&self, source: QString, destination: QString) -> QString {
        let result = (|| -> anyhow::Result<serde_json::Value> {
            use anyhow::Context;
            let catalog = crate::controller_catalog::catalog();
            let source = catalog
                .layout(&source.to_string())
                .context("Unknown source layout")?;
            let destination = catalog
                .layout(&destination.to_string())
                .context("Unknown destination layout")?;
            let available = source
                .controls
                .iter()
                .filter(|control| control.repeat_of.is_none())
                .map(|control| control.id.as_str())
                .collect();
            let requested = destination
                .controls
                .iter()
                .filter(|control| control.repeat_of.is_none())
                .map(|control| control.id.as_str())
                .collect();
            let resolution =
                crate::controller_layout::resolve(source, destination, &available, &requested);
            let rows: Vec<_> = destination.controls.iter().filter(|control| control.repeat_of.is_none()).map(|control| {
                let physical = resolution.assignments.get(&control.id)
                    .and_then(|id| source.controls.iter().find(|candidate| candidate.id == *id));
                serde_json::json!({"target_id":control.id,"target":control.label,
                    "physical_id":physical.map(|control| &control.id),
                    "physical":physical.map(|control| control.label.as_str()).unwrap_or("Unmapped"),
                    "reason":resolution.rules.get(&control.id).map(|rule| rule.description())
                        .or_else(|| resolution.missing.get(&control.id).map(|missing| missing.description())).unwrap_or("No assignment"),
                    "output":""})
            }).collect();
            Ok(serde_json::json!({"rows":rows,"error":"","policy":resolution.policy_version}))
        })();
        qstring(match result {
            Ok(value) => value.to_string(),
            Err(error) => serde_json::json!({"rows":[],"error":error.to_string()}).to_string(),
        })
    }

    pub fn save_controller_name(
        mut self: Pin<&mut Self>,
        device: QString,
        name: QString,
    ) -> QString {
        let id = device.to_string();
        let name = name.to_string().trim().to_owned();
        let result = (|| -> anyhow::Result<()> {
            anyhow::ensure!(!*self.as_ref().busy(), "Wait for settings to finish saving");
            let count = self
                .as_ref()
                .rust()
                .controller_inventory
                .as_ref()
                .map(|inventory| {
                    inventory
                        .controllers
                        .iter()
                        .filter(|device| device.stable_id == id)
                        .count()
                })
                .unwrap_or(0);
            anyhow::ensure!(
                count == 1,
                "Reconnect this controller; its identity is missing or ambiguous"
            );
            SettingsStore::open_default()?.save_controller_name(&id, &name)
        })();
        match result {
            Ok(()) => {
                if name.is_empty() {
                    self.as_mut()
                        .rust_mut()
                        .controller_mapping
                        .device_names
                        .remove(&id);
                } else {
                    self.as_mut()
                        .rust_mut()
                        .controller_mapping
                        .device_names
                        .insert(id, name);
                }
                self.as_mut().bump_controller_revision();
                qstring("")
            }
            Err(error) => qstring(error.to_string()),
        }
    }

    pub fn controller_model_review(&self, device: QString, query: QString) -> QString {
        let id = device.to_string();
        let Some(device) = self
            .rust()
            .controller_inventory
            .as_ref()
            .and_then(|i| i.controllers.iter().find(|d| d.stable_id == id))
        else {
            return qstring(r#"{"candidates":[],"message":"Reconnect this controller"}"#);
        };
        qstring(
            crate::controller_models::review(
                device,
                self.rust()
                    .controller_mapping
                    .device_models
                    .get(&id)
                    .map(String::as_str),
                &query.to_string(),
                std::env::consts::OS,
            )
            .to_string(),
        )
    }

    pub fn save_controller_model(
        mut self: Pin<&mut Self>,
        device: QString,
        model: QString,
    ) -> QString {
        let id = device.to_string();
        let model = model.to_string();
        let result = (|| -> anyhow::Result<()> {
            anyhow::ensure!(!*self.as_ref().busy(), "Wait for settings to finish saving");
            let count = self
                .rust()
                .controller_inventory
                .as_ref()
                .map(|i| i.controllers.iter().filter(|d| d.stable_id == id).count())
                .unwrap_or(0);
            anyhow::ensure!(
                count == 1,
                "Reconnect this controller; its identity is missing or ambiguous"
            );
            SettingsStore::open_default()?.save_controller_model(&id, &model)
        })();
        match result {
            Ok(()) => {
                if model.is_empty() {
                    self.as_mut()
                        .rust_mut()
                        .controller_mapping
                        .device_models
                        .remove(&id);
                } else {
                    self.as_mut()
                        .rust_mut()
                        .controller_mapping
                        .device_models
                        .insert(id, model);
                }
                self.as_mut().bump_controller_revision();
                qstring("")
            }
            Err(error) => qstring(error.to_string()),
        }
    }

    pub fn rename_controller(mut self: Pin<&mut Self>, index: i32, name: QString) {
        let Some(device) = self.as_ref().controller_at(index) else {
            return;
        };
        let name = name.to_string().trim().to_owned();
        if name.chars().count() > 80 || name.chars().any(char::is_control) {
            self.as_mut().set_controller_status(qstring(
                "Use a controller name of up to 80 printable characters.",
            ));
            return;
        }
        let previous = self
            .as_ref()
            .rust()
            .controller_mapping
            .device_names
            .get(&device.stable_id)
            .cloned()
            .unwrap_or_default();
        if name == previous {
            return;
        }
        if name.is_empty() {
            self.as_mut()
                .rust_mut()
                .controller_mapping
                .device_names
                .remove(&device.stable_id);
        } else {
            self.as_mut()
                .rust_mut()
                .controller_mapping
                .device_names
                .insert(device.stable_id, name);
        }
        self.as_mut().controller_settings_changed();
    }

    pub fn controller_detail_at(&self, index: i32) -> QString {
        self.controller_at(index)
            .map(|controller| {
                let vid_pid = match (
                    controller.vendor_id.as_deref(),
                    controller.product_id.as_deref(),
                ) {
                    (Some(vendor), Some(product)) => format!("VID:PID {vendor}:{product}"),
                    _ => "VID:PID unknown".to_owned(),
                };
                let serial = controller
                    .unique_id
                    .as_deref()
                    .map(str::trim)
                    .filter(|value| !value.is_empty())
                    .unwrap_or("no serial");
                qstring(format!("{vid_pid} · {serial} · {}", controller.stable_id))
            })
            .unwrap_or_default()
    }

    pub fn controller_receives_input(&self, index: i32, key: QString) -> bool {
        let Some(controller) = self.controller_at(index) else {
            return false;
        };
        let Some(inventory) = &self.rust().controller_inventory else {
            return false;
        };
        controllers::controller_receives_input(
            &inventory.controllers,
            &controller.stable_id,
            &key.to_string(),
        )
    }

    pub fn controller_action_at(&self, index: i32) -> QString {
        self.controller_at(index)
            .map(|controller| {
                qstring(controllers::controller_action(
                    &self.rust().controller_mapping,
                    &controller.stable_id,
                ))
            })
            .unwrap_or_default()
    }

    pub fn controller_profile_at(&self, index: i32) -> QString {
        self.controller_at(index)
            .map(|controller| {
                qstring(controllers::controller_profile(
                    &self.rust().controller_mapping,
                    &controller.stable_id,
                ))
            })
            .unwrap_or_default()
    }

    pub fn controller_target_at(&self, index: i32) -> QString {
        self.controller_at(index)
            .map(|controller| {
                qstring(controllers::controller_target(
                    &self.rust().controller_mapping,
                    &controller.stable_id,
                ))
            })
            .unwrap_or_default()
    }

    pub fn move_controller(mut self: Pin<&mut Self>, index: i32, direction: i32) {
        let Some(controller_id) = self
            .as_ref()
            .controller_at(index)
            .map(|controller| controller.stable_id)
        else {
            return;
        };
        let Some(inventory) = self.as_ref().rust().controller_inventory.clone() else {
            return;
        };
        if controllers::move_controller(
            &inventory,
            &mut self.as_mut().rust_mut().controller_mapping,
            &controller_id,
            direction as isize,
        ) {
            self.as_mut().controller_settings_changed();
        }
    }

    pub fn choose_controller_action(mut self: Pin<&mut Self>, index: i32, action: QString) {
        let Some(controller_id) = self
            .as_ref()
            .controller_at(index)
            .map(|controller| controller.stable_id)
        else {
            return;
        };
        if controllers::set_controller_action(
            &mut self.as_mut().rust_mut().controller_mapping,
            &controller_id,
            action.to_string().trim(),
        ) {
            self.as_mut().set_controller_enabled(true);
            self.as_mut().controller_settings_changed();
        }
    }

    pub fn choose_controller_profile(mut self: Pin<&mut Self>, index: i32, profile: QString) {
        let Some(controller_id) = self
            .as_ref()
            .controller_at(index)
            .map(|controller| controller.stable_id)
        else {
            return;
        };
        if profile.to_string().trim() == controllers::PROFILE_CREATE {
            self.as_mut()
                .open_new_controller_profile(Some(controller_id));
            return;
        }
        if controllers::set_controller_profile(
            &mut self.as_mut().rust_mut().controller_mapping,
            &controller_id,
            profile.to_string().trim(),
        ) {
            self.as_mut().set_controller_enabled(true);
            self.as_mut().controller_settings_changed();
        }
    }

    pub fn choose_controller_target(mut self: Pin<&mut Self>, index: i32, target: QString) {
        let Some(controller_id) = self
            .as_ref()
            .controller_at(index)
            .map(|controller| controller.stable_id)
        else {
            return;
        };
        let Some(inventory) = self.as_ref().rust().controller_inventory.clone() else {
            return;
        };
        if controllers::set_controller_target(
            &inventory,
            &mut self.as_mut().rust_mut().controller_mapping,
            &controller_id,
            target.to_string().trim(),
        ) {
            self.as_mut().set_controller_enabled(true);
            self.as_mut().controller_settings_changed();
        }
    }

    pub fn controller_profile_count(&self) -> i32 {
        let count = 3
            + controllers::built_in_profiles().len()
            + self.rust().controller_mapping.custom_profiles.len();
        i32::try_from(count).unwrap_or(i32::MAX)
    }

    pub fn controller_profile_id_at(&self, index: i32) -> QString {
        controller_profile_option(&self.rust().controller_mapping, index)
            .map(|(id, _)| qstring(id))
            .unwrap_or_default()
    }

    pub fn controller_profile_name_at(&self, index: i32) -> QString {
        controller_profile_option(&self.rust().controller_mapping, index)
            .map(|(_, name)| qstring(name))
            .unwrap_or_default()
    }

    pub fn controller_target_count(&self) -> i32 {
        self.rust()
            .controller_inventory
            .as_ref()
            .map(|inventory| inventory.supported_targets.len().saturating_add(1))
            .and_then(|count| i32::try_from(count).ok())
            .unwrap_or(1)
    }

    pub fn controller_target_id_at(&self, index: i32) -> QString {
        if index == 0 {
            return qstring(controllers::TARGET_INHERIT);
        }
        usize::try_from(index - 1)
            .ok()
            .and_then(|index| {
                self.rust()
                    .controller_inventory
                    .as_ref()?
                    .supported_targets
                    .get(index)
            })
            .map(|target| qstring(&target.id))
            .unwrap_or_default()
    }

    pub fn controller_target_name_at(&self, index: i32) -> QString {
        if index == 0 {
            return qstring("Default target");
        }
        usize::try_from(index - 1)
            .ok()
            .and_then(|index| {
                self.rust()
                    .controller_inventory
                    .as_ref()?
                    .supported_targets
                    .get(index)
            })
            .map(|target| qstring(&target.name))
            .unwrap_or_default()
    }

    pub fn custom_controller_profile_count(&self) -> i32 {
        i32::try_from(self.rust().controller_mapping.custom_profiles.len()).unwrap_or(i32::MAX)
    }

    pub fn custom_controller_profile_id_at(&self, index: i32) -> QString {
        self.custom_controller_profile_at(index)
            .map(|profile| qstring(&profile.id))
            .unwrap_or_default()
    }

    pub fn custom_controller_profile_name_at(&self, index: i32) -> QString {
        self.custom_controller_profile_at(index)
            .map(|profile| qstring(&profile.name))
            .unwrap_or_default()
    }

    pub fn custom_controller_profile_detail_at(&self, index: i32) -> QString {
        self.custom_controller_profile_at(index)
            .map(|profile| {
                let layout = match profile.layout.as_str() {
                    "playstation" => "PlayStation",
                    "generic" => "Generic",
                    _ => "Xbox",
                };
                let mapping_label = match profile.mappings.len() {
                    0 => "identity mapping".to_owned(),
                    1 => "1 remapped button".to_owned(),
                    count => format!("{count} remapped buttons"),
                };
                qstring(format!("{layout} diagram · {mapping_label}"))
            })
            .unwrap_or_default()
    }

    pub fn create_controller_profile(mut self: Pin<&mut Self>) {
        self.as_mut().open_new_controller_profile(None);
    }

    pub fn edit_controller_profile(mut self: Pin<&mut Self>, index: i32) {
        let Some(profile) = self.as_ref().custom_controller_profile_at(index).cloned() else {
            return;
        };
        self.as_mut()
            .set_controller_profile_editor_id(qstring(&profile.id));
        self.as_mut()
            .set_controller_profile_editor_name(qstring(&profile.name));
        self.as_mut()
            .set_controller_profile_editor_layout(qstring(&profile.layout));
        self.as_mut()
            .set_controller_profile_editor_target(qstring("South"));
        self.as_mut()
            .set_controller_profile_editor_status(qstring("Edit the diagram, then save."));
        self.as_mut().rust_mut().controller_profile_editor_mappings = profile.mappings;
        self.as_mut()
            .rust_mut()
            .controller_profile_pending_controller_id = None;
        self.as_mut().set_controller_profile_editor_open(true);
        self.as_mut().bump_controller_profile_revision();
    }

    pub fn delete_controller_profile(mut self: Pin<&mut Self>, index: i32) {
        let Some(profile) = self.as_ref().custom_controller_profile_at(index).cloned() else {
            return;
        };
        if !controllers::remove_custom_profile(
            &mut self.as_mut().rust_mut().controller_mapping,
            &profile.id,
        ) {
            return;
        }
        if self.as_ref().controller_profile_editor_id().to_string() == profile.id {
            self.as_mut().close_controller_profile_editor();
        }
        self.as_mut()
            .set_controller_profile_editor_status(qstring(format!(
                "Deleted profile {:?} and cleared its launch assignments.",
                profile.name
            )));
        self.as_mut().controller_settings_changed();
        self.as_mut().bump_controller_profile_revision();
    }

    pub fn cancel_controller_profile_edit(mut self: Pin<&mut Self>) {
        self.as_mut().close_controller_profile_editor();
        self.as_mut().bump_controller_revision();
    }

    pub fn save_controller_profile(mut self: Pin<&mut Self>) {
        let name = self
            .as_ref()
            .controller_profile_editor_name()
            .to_string()
            .trim()
            .to_owned();
        if name.is_empty() || name.chars().count() > 100 {
            self.as_mut().set_controller_profile_editor_status(qstring(
                "Profile name must contain 1 to 100 characters.",
            ));
            return;
        }
        let layout = self.as_ref().controller_profile_editor_layout().to_string();
        if !matches!(layout.as_str(), "xbox" | "playstation" | "generic") {
            self.as_mut().set_controller_profile_editor_status(qstring(
                "Choose the Xbox, PlayStation, or generic diagram.",
            ));
            return;
        }
        let saved_id = self.as_ref().controller_profile_editor_id().to_string();
        let id = if saved_id.trim().is_empty() {
            format!("custom:{}", uuid::Uuid::new_v4())
        } else {
            saved_id
        };
        let profile = ControllerCustomProfile {
            id: id.clone(),
            name: name.clone(),
            layout,
            mappings: self
                .as_ref()
                .rust()
                .controller_profile_editor_mappings
                .clone(),
        };
        let pending_controller_id = self
            .as_ref()
            .rust()
            .controller_profile_pending_controller_id
            .clone();
        let mut candidate = self.as_ref().rust().controller_mapping.clone();
        if let Some(index) = candidate
            .custom_profiles
            .iter()
            .position(|saved| saved.id == id)
        {
            candidate.custom_profiles[index] = profile;
        } else {
            candidate.custom_profiles.push(profile);
        }
        if let Some(controller_id) = pending_controller_id.as_deref()
            && !controllers::set_controller_profile(&mut candidate, controller_id, &id)
        {
            self.as_mut().set_controller_profile_editor_status(qstring(
                "The new profile could not be assigned to the selected controller.",
            ));
            return;
        }
        if let Err(error) = candidate.validate() {
            self.as_mut()
                .set_controller_profile_editor_status(qstring(error.to_string()));
            return;
        }
        self.as_mut().rust_mut().controller_mapping = candidate;
        if pending_controller_id.is_some() {
            self.as_mut().set_controller_enabled(true);
        }
        self.as_mut().close_controller_profile_editor();
        self.as_mut()
            .set_controller_profile_editor_status(qstring(format!(
                "Saved custom controller profile {name:?}."
            )));
        self.as_mut().controller_settings_changed();
        self.as_mut().bump_controller_profile_revision();
    }

    pub fn select_controller_profile_target(mut self: Pin<&mut Self>, target: QString) {
        let target = target.to_string();
        if !controllers::CONTROLLER_PROFILE_BUTTONS
            .iter()
            .any(|(button, _)| *button == target)
        {
            return;
        }
        self.as_mut()
            .set_controller_profile_editor_target(qstring(target));
        self.as_mut()
            .set_controller_profile_editor_status(QString::default());
        self.as_mut().bump_controller_profile_revision();
    }

    pub fn controller_profile_button_count(&self) -> i32 {
        i32::try_from(controllers::CONTROLLER_PROFILE_BUTTONS.len()).unwrap_or(i32::MAX)
    }

    pub fn controller_profile_button_id_at(&self, index: i32) -> QString {
        usize::try_from(index)
            .ok()
            .and_then(|index| controllers::CONTROLLER_PROFILE_BUTTONS.get(index))
            .map(|(id, _)| qstring(id))
            .unwrap_or_default()
    }

    pub fn controller_profile_button_name_at(&self, index: i32) -> QString {
        usize::try_from(index)
            .ok()
            .and_then(|index| controllers::CONTROLLER_PROFILE_BUTTONS.get(index))
            .map(|(_, name)| qstring(name))
            .unwrap_or_default()
    }

    pub fn controller_profile_button_label(&self, button: QString) -> QString {
        qstring(controllers::controller_button_label(
            &self.controller_profile_editor_layout().to_string(),
            button.to_string().trim(),
        ))
    }

    pub fn controller_profile_selected_source(&self) -> QString {
        let target = self.controller_profile_editor_target().to_string();
        let profile = ControllerCustomProfile {
            mappings: self.rust().controller_profile_editor_mappings.clone(),
            ..ControllerCustomProfile::default()
        };
        qstring(controllers::custom_profile_source_for_target(
            &profile, &target,
        ))
    }

    pub fn choose_controller_profile_source(mut self: Pin<&mut Self>, source: QString) {
        let target = self.as_ref().controller_profile_editor_target().to_string();
        let source = source.to_string();
        let mut profile = ControllerCustomProfile {
            mappings: self
                .as_ref()
                .rust()
                .controller_profile_editor_mappings
                .clone(),
            ..ControllerCustomProfile::default()
        };
        if controllers::set_custom_profile_mapping(&mut profile, &target, &source) {
            self.as_mut().rust_mut().controller_profile_editor_mappings = profile.mappings;
            self.as_mut()
                .set_controller_profile_editor_status(QString::default());
            self.as_mut().bump_controller_profile_revision();
        }
    }

    pub fn controller_profile_mapping_count(&self) -> i32 {
        i32::try_from(self.rust().controller_profile_editor_mappings.len()).unwrap_or(i32::MAX)
    }

    pub fn controller_profile_mapping_source_at(&self, index: i32) -> QString {
        usize::try_from(index)
            .ok()
            .and_then(|index| self.rust().controller_profile_editor_mappings.get(index))
            .map(|mapping| qstring(&mapping.source_button))
            .unwrap_or_default()
    }

    pub fn controller_profile_mapping_target_at(&self, index: i32) -> QString {
        usize::try_from(index)
            .ok()
            .and_then(|index| self.rust().controller_profile_editor_mappings.get(index))
            .map(|mapping| qstring(&mapping.target_button))
            .unwrap_or_default()
    }

    pub fn apply_two_button_controller_preset(mut self: Pin<&mut Self>) {
        let mut profile = ControllerCustomProfile::default();
        controllers::apply_two_button_profile_preset(&mut profile);
        self.as_mut().rust_mut().controller_profile_editor_mappings = profile.mappings;
        self.as_mut().set_controller_profile_editor_status(qstring(
            "Applied the 2-button X/A preset. Identity mappings remain implicit.",
        ));
        self.as_mut().bump_controller_profile_revision();
        if std::env::args().any(|argument| argument == "--controller-profile-ui-probe") {
            println!(
                "LUNCHBOX_CONTROLLER_PROFILE_UI_READY buttons={} mappings={}",
                controllers::CONTROLLER_PROFILE_BUTTONS.len(),
                self.as_ref()
                    .rust()
                    .controller_profile_editor_mappings
                    .len()
            );
        }
    }

    pub fn clear_controller_profile_mappings(mut self: Pin<&mut Self>) {
        self.as_mut()
            .rust_mut()
            .controller_profile_editor_mappings
            .clear();
        self.as_mut().set_controller_profile_editor_status(qstring(
            "Cleared every remap; the profile now passes buttons through by identity.",
        ));
        self.as_mut().bump_controller_profile_revision();
    }

    pub fn set_directory(mut self: Pin<&mut Self>, field: QString, url: QUrl) {
        let Some(path) = url.to_local_file() else {
            self.as_mut()
                .set_message(qstring("Choose a local filesystem directory."));
            return;
        };
        match field.to_string().as_str() {
            "rom" => self.as_mut().set_rom_directory(path),
            "torrent" => self.as_mut().set_torrent_library_directory(path),
            "torrent-watch" => self.as_mut().set_watched_torrent_directory(path),
            "torrent-watch-archive" => self.as_mut().set_watched_torrent_archive_directory(path),
            _ => self
                .as_mut()
                .set_message(qstring("Unknown settings directory field.")),
        }
    }

    pub fn choose_native_directory(mut self: Pin<&mut Self>, field: QString) {
        let field = field.to_string();
        let current = match field.as_str() {
            "rom" => self.as_ref().rom_directory().to_string(),
            "torrent" => self.as_ref().torrent_library_directory().to_string(),
            "torrent-watch" => self.as_ref().watched_torrent_directory().to_string(),
            "torrent-watch-archive" => self
                .as_ref()
                .watched_torrent_archive_directory()
                .to_string(),
            _ => {
                self.as_mut()
                    .set_message(qstring("Unknown settings directory field."));
                return;
            }
        };
        let mut dialog = rfd::FileDialog::new().set_title(match field.as_str() {
            "rom" => "Choose ROM library",
            "torrent" => "Choose torrent download library",
            "torrent-watch" => "Choose watched torrent inbox",
            _ => "Choose archive for added torrent metadata",
        });
        if !current.trim().is_empty() {
            dialog = dialog.set_directory(&current);
        }
        let Some(path) = dialog.pick_folder() else {
            return;
        };
        match field.as_str() {
            "rom" => self
                .as_mut()
                .set_rom_directory(qstring(path.to_string_lossy())),
            "torrent" => self
                .as_mut()
                .set_torrent_library_directory(qstring(path.to_string_lossy())),
            "torrent-watch" => self
                .as_mut()
                .set_watched_torrent_directory(qstring(path.to_string_lossy())),
            "torrent-watch-archive" => self
                .as_mut()
                .set_watched_torrent_archive_directory(qstring(path.to_string_lossy())),
            _ => unreachable!(),
        }
        self.as_mut().set_message(qstring(
            "Directory selected. Save settings to keep this native path.",
        ));
    }

    pub fn export_profile(mut self: Pin<&mut Self>, destination: QUrl) {
        if *self.as_ref().profile_busy() {
            return;
        }
        let Some(destination) = destination
            .to_local_file()
            .map(|path| PathBuf::from(path.to_string()))
        else {
            self.as_mut()
                .set_profile_message(qstring("Choose a local profile backup file."));
            return;
        };
        let state_path = match settings::state_database_path() {
            Ok(path) => path,
            Err(error) => {
                self.as_mut().set_profile_message(qstring(format!(
                    "Could not locate the Lunchbox profile: {error}"
                )));
                return;
            }
        };
        self.as_mut().set_profile_busy(true);
        self.as_mut().set_profile_restore_ready(false);
        self.as_mut()
            .set_profile_message(qstring("Creating a consistent profile snapshot…"));
        let qt_thread = self.as_ref().qt_thread();
        let spawn = std::thread::Builder::new()
            .name("lunchbox-profile-export".into())
            .spawn(move || {
                let result = profile_backup::export_profile(&state_path, &destination)
                    .map(|summary| (summary, destination))
                    .map_err(|error| error.to_string());
                let _ = qt_thread.queue(move |mut model| {
                    model.as_mut().set_profile_busy(false);
                    match result {
                        Ok((summary, destination)) => model.as_mut().set_profile_message(qstring(
                            format!(
                                "Backup verified and saved to {}: {}. Credential-store secrets, ROMs, downloads, media caches, and emulator binaries were not included.",
                                destination.display(),
                                summary.description()
                            ),
                        )),
                        Err(error) => model.as_mut().set_profile_message(qstring(format!(
                            "Profile backup failed: {error}"
                        ))),
                    }
                });
            });
        if let Err(error) = spawn {
            self.as_mut().set_profile_busy(false);
            self.as_mut().set_profile_message(qstring(format!(
                "Could not start the profile backup worker: {error}"
            )));
        }
    }

    pub fn inspect_profile(mut self: Pin<&mut Self>, source: QUrl) {
        if *self.as_ref().profile_busy() {
            return;
        }
        let Some(source) = source
            .to_local_file()
            .map(|path| PathBuf::from(path.to_string()))
        else {
            self.as_mut()
                .set_profile_message(qstring("Choose a local Lunchbox profile archive."));
            return;
        };
        self.as_mut().rust_mut().profile_restore_source = None;
        self.as_mut().set_profile_restore_ready(false);
        self.as_mut()
            .set_profile_restore_summary(QString::default());
        self.as_mut().set_profile_busy(true);
        self.as_mut().set_profile_message(qstring(
            "Validating the profile manifest, receipts, database, and themes…",
        ));
        let qt_thread = self.as_ref().qt_thread();
        let spawn = std::thread::Builder::new()
            .name("lunchbox-profile-inspect".into())
            .spawn(move || {
                let result = profile_backup::inspect_profile(&source)
                    .map(|summary| (summary, source))
                    .map_err(|error| error.to_string());
                let _ = qt_thread.queue(move |mut model| {
                    model.as_mut().set_profile_busy(false);
                    match result {
                        Ok((summary, source)) => {
                            let description = summary.description();
                            model.as_mut().rust_mut().profile_restore_source = Some(source);
                            model
                                .as_mut()
                                .set_profile_restore_summary(qstring(&description));
                            model.as_mut().set_profile_message(qstring(format!(
                                "Verified profile archive: {description}. Review the replacement before staging it."
                            )));
                            model.as_mut().set_profile_restore_ready(true);
                        }
                        Err(error) => model.as_mut().set_profile_message(qstring(format!(
                            "Profile archive was rejected: {error}"
                        ))),
                    }
                });
            });
        if let Err(error) = spawn {
            self.as_mut().set_profile_busy(false);
            self.as_mut().set_profile_message(qstring(format!(
                "Could not start profile validation: {error}"
            )));
        }
    }

    pub fn stage_profile_restore(mut self: Pin<&mut Self>) {
        if *self.as_ref().profile_busy() {
            return;
        }
        let Some(source) = self.as_ref().rust().profile_restore_source.clone() else {
            self.as_mut()
                .set_profile_message(qstring("Choose and verify a profile archive first."));
            return;
        };
        let state_path = match settings::state_database_path() {
            Ok(path) => path,
            Err(error) => {
                self.as_mut().set_profile_message(qstring(format!(
                    "Could not locate the Lunchbox profile: {error}"
                )));
                return;
            }
        };
        self.as_mut().set_profile_busy(true);
        self.as_mut().set_profile_restore_ready(false);
        self.as_mut()
            .set_profile_message(qstring("Revalidating and staging the profile restore…"));
        let qt_thread = self.as_ref().qt_thread();
        let spawn = std::thread::Builder::new()
            .name("lunchbox-profile-stage".into())
            .spawn(move || {
                let result = profile_backup::stage_restore(&state_path, &source)
                    .map_err(|error| error.to_string());
                let _ = qt_thread.queue(move |mut model| {
                    model.as_mut().set_profile_busy(false);
                    match result {
                        Ok(summary) => {
                            model.as_mut().rust_mut().profile_restore_source = None;
                            model
                                .as_mut()
                                .set_profile_restore_summary(qstring(summary.description()));
                            model.as_mut().set_profile_restart_required(true);
                            model.as_mut().set_profile_message(qstring(
                                "Restore staged safely. Quit Lunchbox to apply it before any models open on the next launch.",
                            ));
                        }
                        Err(error) => model.as_mut().set_profile_message(qstring(format!(
                            "Profile restore was not staged: {error}"
                        ))),
                    }
                });
            });
        if let Err(error) = spawn {
            self.as_mut().set_profile_busy(false);
            self.as_mut().set_profile_message(qstring(format!(
                "Could not start the profile restore worker: {error}"
            )));
        }
    }

    pub fn cancel_profile_restore(mut self: Pin<&mut Self>) {
        if *self.as_ref().profile_busy() {
            return;
        }
        self.as_mut().rust_mut().profile_restore_source = None;
        self.as_mut().set_profile_restore_ready(false);
        self.as_mut()
            .set_profile_restore_summary(QString::default());
        self.as_mut().set_profile_message(qstring(
            "Profile restore review cancelled; nothing was changed.",
        ));
    }

    pub fn cancel_staged_profile_restore(mut self: Pin<&mut Self>) {
        if *self.as_ref().profile_busy() || !*self.as_ref().profile_restart_required() {
            return;
        }
        let state_path = match settings::state_database_path() {
            Ok(path) => path,
            Err(error) => {
                self.as_mut().set_profile_message(qstring(format!(
                    "Could not locate the staged Lunchbox profile: {error}"
                )));
                return;
            }
        };
        self.as_mut().set_profile_busy(true);
        self.as_mut()
            .set_profile_message(qstring("Cancelling the staged profile restore safely…"));
        let qt_thread = self.as_ref().qt_thread();
        let spawn = std::thread::Builder::new()
            .name("lunchbox-profile-unstage".into())
            .spawn(move || {
                let result = profile_backup::cancel_pending_restore(&state_path)
                    .map_err(|error| error.to_string());
                let _ = qt_thread.queue(move |mut model| {
                    model.as_mut().set_profile_busy(false);
                    match result {
                        Ok(removed) => {
                            model.as_mut().set_profile_restart_required(false);
                            model
                                .as_mut()
                                .set_profile_restore_summary(QString::default());
                            model.as_mut().set_profile_message(qstring(if removed {
                                "Staged profile restore cancelled; the current profile was never changed."
                            } else {
                                "No staged profile restore remained; the current profile is unchanged."
                            }));
                        }
                        Err(error) => model.as_mut().set_profile_message(qstring(format!(
                            "Could not cancel the staged profile restore: {error}"
                        ))),
                    }
                });
            });
        if let Err(error) = spawn {
            self.as_mut().set_profile_busy(false);
            self.as_mut().set_profile_message(qstring(format!(
                "Could not start the profile restore cancellation worker: {error}"
            )));
        }
    }

    fn apply_settings(mut self: Pin<&mut Self>, settings: AppSettings) {
        let region_priority =
            crate::region_priority::effective_region_priority(&settings.region_priority);
        let media_provider_priority =
            crate::media::effective_provider_priority(&settings.media_provider_priority);
        let controller_mapping = settings.controller_mapping.clone();
        self.as_mut()
            .set_onboarding_complete(settings.onboarding_complete);
        self.as_mut()
            .set_qbittorrent_host(qstring(settings.qbittorrent_host));
        self.as_mut()
            .set_qbittorrent_port(i32::from(settings.qbittorrent_port));
        self.as_mut()
            .set_qbittorrent_use_https(settings.qbittorrent_use_https);
        self.as_mut()
            .set_qbittorrent_username(qstring(settings.qbittorrent_username));
        self.as_mut()
            .set_rom_directory(qstring(settings.rom_directory.to_string_lossy()));
        self.as_mut()
            .set_qbittorrent_container_rom_directory(qstring(
                settings.qbittorrent_container_rom_directory,
            ));
        self.as_mut().set_torrent_library_directory(qstring(
            settings.torrent_library_directory.to_string_lossy(),
        ));
        self.as_mut()
            .set_qbittorrent_container_torrent_library_directory(qstring(
                settings.qbittorrent_container_torrent_library_directory,
            ));
        self.as_mut().set_watched_torrent_directory(qstring(
            settings.watched_torrent_directory.to_string_lossy(),
        ));
        self.as_mut().set_watched_torrent_archive_directory(qstring(
            settings.watched_torrent_archive_directory.to_string_lossy(),
        ));
        self.as_mut()
            .set_download_entire_torrent(settings.download_entire_torrent);
        self.as_mut()
            .set_minimize_during_game(settings.minimize_during_game);
        self.as_mut()
            .set_file_link_mode(qstring(settings.file_link_mode));
        self.as_mut()
            .set_seeding_policy(qstring(settings.seeding_policy));
        self.as_mut().rust_mut().region_priority = region_priority;
        self.as_mut().sync_primary_region();
        self.as_mut().bump_region_revision();
        self.as_mut()
            .set_version_preference(qstring(settings.version_preference));
        self.as_mut().rust_mut().media_provider_priority = media_provider_priority;
        self.as_mut().bump_media_provider_revision();
        self.as_mut()
            .set_controller_enabled(controller_mapping.enabled);
        self.as_mut()
            .set_controller_automatic(controller_mapping.automatic);
        self.as_mut()
            .set_controller_calibrated_launch(controller_mapping.calibrated_launch);
        self.as_mut()
            .set_controller_output_target(qstring(&controller_mapping.output_target));
        self.as_mut().rust_mut().controller_mapping = controller_mapping;
        self.as_mut().close_controller_profile_editor();
        self.as_mut().bump_controller_revision();
        self.as_mut().bump_controller_profile_revision();
    }

    fn settings_snapshot(&self) -> Result<AppSettings, String> {
        let port = u16::try_from(*self.qbittorrent_port())
            .map_err(|_| "qBittorrent port must be between 1 and 65535".to_owned())?;
        let settings = AppSettings {
            onboarding_complete: *self.onboarding_complete(),
            minimize_during_game: *self.minimize_during_game(),
            qbittorrent_host: self.qbittorrent_host().to_string(),
            qbittorrent_port: port,
            qbittorrent_use_https: *self.qbittorrent_use_https(),
            qbittorrent_username: self.qbittorrent_username().to_string(),
            rom_directory: PathBuf::from(self.rom_directory().to_string()),
            qbittorrent_container_rom_directory: self
                .qbittorrent_container_rom_directory()
                .to_string(),
            torrent_library_directory: PathBuf::from(self.torrent_library_directory().to_string()),
            qbittorrent_container_torrent_library_directory: self
                .qbittorrent_container_torrent_library_directory()
                .to_string(),
            watched_torrent_directory: PathBuf::from(self.watched_torrent_directory().to_string()),
            watched_torrent_archive_directory: PathBuf::from(
                self.watched_torrent_archive_directory().to_string(),
            ),
            download_entire_torrent: *self.download_entire_torrent(),
            file_link_mode: self.file_link_mode().to_string(),
            seeding_policy: self.seeding_policy().to_string(),
            preferred_region: self.preferred_region().to_string(),
            region_priority: self.rust().region_priority.clone(),
            version_preference: self.version_preference().to_string(),
            media_provider_priority: self.rust().media_provider_priority.clone(),
            controller_mapping: {
                let mut mapping = self.rust().controller_mapping.clone();
                mapping.enabled = *self.controller_enabled();
                mapping.output_target = self.controller_output_target().to_string();
                mapping
            },
        };
        settings.validate().map_err(|error| error.to_string())?;
        Ok(settings)
    }

    fn sync_primary_region(mut self: Pin<&mut Self>) {
        let primary = self
            .as_ref()
            .rust()
            .region_priority
            .iter()
            .find(|region| !region.is_empty())
            .cloned()
            .unwrap_or_else(|| "USA".to_owned());
        let legacy_primary = if matches!(
            primary.as_str(),
            "USA" | "Japan" | "Europe" | "World" | "Asia"
        ) {
            primary
        } else {
            "any".to_owned()
        };
        self.as_mut().set_preferred_region(qstring(legacy_primary));
    }

    fn bump_region_revision(mut self: Pin<&mut Self>) {
        let revision = self.as_ref().region_revision().wrapping_add(1);
        self.as_mut().set_region_revision(revision);
    }

    fn bump_media_provider_revision(mut self: Pin<&mut Self>) {
        let revision = self.as_ref().media_provider_revision().wrapping_add(1);
        self.as_mut().set_media_provider_revision(revision);
    }

    fn controller_at(&self, index: i32) -> Option<ControllerDevice> {
        let index = usize::try_from(index).ok()?;
        let inventory = self.rust().controller_inventory.as_ref()?;
        controllers::ordered_controllers(inventory, &self.rust().controller_mapping)
            .get(index)
            .cloned()
    }

    fn custom_controller_profile_at(&self, index: i32) -> Option<&ControllerCustomProfile> {
        usize::try_from(index)
            .ok()
            .and_then(|index| self.rust().controller_mapping.custom_profiles.get(index))
    }

    fn open_new_controller_profile(mut self: Pin<&mut Self>, controller_id: Option<String>) {
        self.as_mut()
            .set_controller_profile_editor_id(QString::default());
        self.as_mut()
            .set_controller_profile_editor_name(qstring("Custom profile"));
        self.as_mut()
            .set_controller_profile_editor_layout(qstring("xbox"));
        self.as_mut()
            .set_controller_profile_editor_target(qstring("South"));
        self.as_mut().set_controller_profile_editor_status(qstring(
            "Choose a virtual target on the diagram, then assign its physical source.",
        ));
        self.as_mut()
            .rust_mut()
            .controller_profile_editor_mappings
            .clear();
        self.as_mut()
            .rust_mut()
            .controller_profile_pending_controller_id = controller_id;
        self.as_mut().set_controller_profile_editor_open(true);
        self.as_mut().bump_controller_revision();
        self.as_mut().bump_controller_profile_revision();
        if std::env::args().any(|argument| argument == "--controller-profile-ui-probe") {
            println!(
                "LUNCHBOX_CONTROLLER_PROFILE_UI_OPENED buttons={} mappings=0",
                controllers::CONTROLLER_PROFILE_BUTTONS.len()
            );
        }
    }

    fn close_controller_profile_editor(mut self: Pin<&mut Self>) {
        self.as_mut().set_controller_profile_editor_open(false);
        self.as_mut()
            .rust_mut()
            .controller_profile_pending_controller_id = None;
    }

    fn controller_settings_changed(mut self: Pin<&mut Self>) {
        self.as_mut().bump_controller_revision();
        self.as_mut().set_message(qstring(
            "Controller mapping changed. Save settings to apply it when games launch.",
        ));
    }

    fn bump_controller_revision(mut self: Pin<&mut Self>) {
        let revision = self.as_ref().controller_revision().wrapping_add(1);
        self.as_mut().set_controller_revision(revision);
    }

    fn bump_controller_profile_revision(mut self: Pin<&mut Self>) {
        let revision = self.as_ref().controller_profile_revision().wrapping_add(1);
        self.as_mut().set_controller_profile_revision(revision);
    }
}

fn controller_inventory_status(inventory: &ControllerInventory) -> String {
    let provider = &inventory.provider;
    let mut status = match provider.provider.as_str() {
        "inputplumber" if provider.available && provider.service_accessible => {
            let raw_version = provider.version.as_deref().unwrap_or("unknown version");
            let version = raw_version
                .strip_prefix("inputplumber ")
                .or_else(|| raw_version.strip_prefix("InputPlumber "))
                .unwrap_or(raw_version);
            format!(
                "InputPlumber {version} · {} connected game controllers · {} managed devices · {} virtual targets",
                inventory.controllers.len(),
                inventory.managed_device_count,
                inventory.supported_targets.len()
            )
        }
        "native" => format!(
            "Native gamepad discovery · {} connected game controllers · launch-time remapping requires the Linux InputPlumber adapter.",
            inventory.controllers.len()
        ),
        _ => provider.message.clone().unwrap_or_else(|| {
            "Native controller remapping is not available on this computer.".to_owned()
        }),
    };
    if !inventory.warnings.is_empty() {
        status.push_str(" · ");
        status.push_str(&inventory.warnings.join("; "));
    }
    status
}

fn controller_profile_option(
    mapping: &ControllerMappingSettings,
    index: i32,
) -> Option<(String, String)> {
    match index {
        0 => Some((
            controllers::PROFILE_INHERIT.to_owned(),
            "Use game, system, or default profile".to_owned(),
        )),
        1 => Some((
            controllers::PROFILE_NONE.to_owned(),
            "Off (no button remap)".to_owned(),
        )),
        _ => {
            let index = usize::try_from(index - 2).ok()?;
            let built_in = controllers::built_in_profiles();
            if let Some(profile) = built_in.get(index) {
                return Some((profile.id.clone(), profile.name.clone()));
            }
            let custom_index = index.checked_sub(built_in.len())?;
            if let Some(profile) = mapping.custom_profiles.get(custom_index) {
                return Some((profile.id.clone(), profile.name.clone()));
            }
            (custom_index == mapping.custom_profiles.len()).then(|| {
                (
                    controllers::PROFILE_CREATE.to_owned(),
                    "Create new profile…".to_owned(),
                )
            })
        }
    }
}

fn load_settings() -> anyhow::Result<(
    AppSettings,
    Option<settings::QbittorrentConnectionTest>,
    PathBuf,
    Option<profile_backup::ProfileSummary>,
)> {
    let store = SettingsStore::open_default()?;
    let settings = store.load()?;
    let connection_test = store.qbittorrent_connection_test()?;
    let restored_profile = match profile_backup::take_applied_notice(store.path()) {
        Ok(summary) => summary,
        Err(error) => {
            eprintln!("LUNCHBOX_PROFILE_RESTORE_NOTICE_FAILED error={error:#}");
            None
        }
    };
    Ok((
        settings,
        connection_test,
        store.path().to_owned(),
        restored_profile,
    ))
}

fn save_settings(
    settings: &AppSettings,
    password: &str,
    connection_ok: bool,
) -> anyhow::Result<PathBuf> {
    let store = SettingsStore::open_default()?;
    store.save(settings)?;
    if !password.is_empty() {
        settings::save_password(password)?;
    }
    if !connection_ok {
        store.clear_qbittorrent_connection_test()?;
    }
    Ok(store.path().to_owned())
}

fn test_and_record_connection(
    settings: &AppSettings,
    password: &str,
) -> anyhow::Result<qbittorrent::QbittorrentConnectionDetails> {
    match qbittorrent::test_connection_details(settings, password) {
        Ok(details) => {
            SettingsStore::open_default()?.save_qbittorrent_connection_test(
                settings,
                &details.version,
                !settings.qbittorrent_username.trim().is_empty() || !password.is_empty(),
            )?;
            Ok(details)
        }
        Err(error) => {
            if let Ok(store) = SettingsStore::open_default() {
                let _ = store.clear_qbittorrent_connection_test();
            }
            Err(error)
        }
    }
}

fn effective_password(entered_password: String) -> anyhow::Result<String> {
    if entered_password.is_empty() {
        Ok(settings::load_password()?.unwrap_or_default())
    } else {
        Ok(entered_password)
    }
}
