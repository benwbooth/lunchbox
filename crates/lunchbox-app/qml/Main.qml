pragma ComponentBehavior: Bound

import QtQuick
import QtQuick.Controls
import QtQuick.Dialogs
import QtQuick.Layouts
import QtMultimedia
import Lunchbox

ApplicationWindow {
    id: root
    visible: true
    width: controllerProfileUiProbe || launchProfileUiProbe
           || launchProfileManagerUiProbe ? 1920 : 1440
    height: controllerProfileUiProbe || launchProfileUiProbe
            || launchProfileManagerUiProbe ? 1200 : 900
    minimumWidth: 1040
    minimumHeight: 680
    title: "Lunchbox"
    color: palette.window

    readonly property color ink: "#f4f7fb"
    readonly property color muted: "#8d99aa"
    readonly property color panel: "#131923"
    readonly property color panelRaised: "#1a2230"
    readonly property color line: "#283244"
    readonly property color accent: "#ffb454"
    readonly property color accentCool: "#62d6c6"
    property string selectedPlatform: ""
    property string availability: ""
    property string selectedCollectionId: ""
    property string selectedCollectionName: ""
    property string editingCollectionId: ""
    property string deletingCollectionId: ""
    property string deletingCollectionName: ""
    property string pendingHistoryJobId: ""
    property string pendingHistoryJobTitle: ""
    property bool clearAllDownloadHistory: false
    property int pendingDownloadPlanIndex: -1
    property bool gridMode: true
    property string selectedGameId: ""
    property int selectedDatabaseId: 0
    property url selectedArtworkUrl: ""
    property url selectedFanartUrl: ""
    property url selectedBoxFrontUrl: ""
    property url selectedBoxBackUrl: ""
    property bool selectedBox3d: false
    property string selectedArtworkSource: ""
    property string selectedHeroArtworkType: ""
    property int selectedHeroArtworkIndex: 0
    property int selectedHeroArtworkCount: 0
    readonly property bool selectedFavorite: {
        library.favorite_revision
        return selectedGameId.length > 0 && library.is_favorite(selectedGameId)
    }
    readonly property bool selectedFavoritePending: {
        library.favorite_pending_count
        return selectedGameId.length > 0 && library.favorite_pending(selectedGameId)
    }
    property string directoryTarget: ""
    property bool favoriteProbeArmed: false
    property bool favoriteProbeTriggered: false
    property bool closeAfterFavoriteSave: false
    property int collectionProbeStage: 0
    property bool collectionProbeArmed: false
    property bool downloadHistoryTriggered: false
    property bool settingsSeedingTriggered: false
    property bool settingsReleaseTriggered: false
    property bool box3dProbeOpened: false
    property bool mediaRotationProbeOpened: false
    property int pendingEmulatorUninstallIndex: -1
    property string pendingEmulatorUninstallName: ""
    property int pendingControllerProfileDeleteIndex: -1
    property string pendingControllerProfileDeleteName: ""
    property int downloadPlanProbeBundleIndex: 0
    property bool launchProbeTriggered: false
    property bool launchProbeObservedRunning: false
    property bool launchProbeAwaitingActivity: false
    property bool launchProfileProbeTriggered: false
    property int mediaBundleProbeStage: 0
    property string mediaPlaybackMessage: ""
    readonly property var artworkChoices: [
        { key: "box-front", label: "Box front" },
        { key: "box-back", label: "Box back" },
        { key: "box-3d", label: "3D box" },
        { key: "screenshot", label: "Gameplay" },
        { key: "title-screen", label: "Title screen" },
        { key: "fanart", label: "Fan art" },
        { key: "clear-logo", label: "Clear logo" }
    ]
    readonly property var completionChoices: [
        { key: "not_started", label: "Not started" },
        { key: "in_progress", label: "In progress" },
        { key: "completed", label: "Completed" },
        { key: "on_hold", label: "On hold" },
        { key: "abandoned", label: "Abandoned" }
    ]
    readonly property int activeFilterCount: (availability.length > 0 ? 1 : 0)
                                               + (library.hide_non_retail ? 1 : 0)
                                               + (library.hide_adult ? 1 : 0)
    readonly property bool archiveImportUiProbe: Qt.application.arguments.indexOf("--archive-import-ui-probe") >= 0
    readonly property bool importUiProbe: Qt.application.arguments.indexOf("--import-ui-probe") >= 0
                                          || archiveImportUiProbe
    readonly property bool importCommitProbe: Qt.application.arguments.indexOf("--import-commit-probe") >= 0
    readonly property bool filterUiProbe: Qt.application.arguments.indexOf("--filter-ui-probe") >= 0
    readonly property bool artworkUiProbe: Qt.application.arguments.indexOf("--artwork-ui-probe") >= 0
    readonly property bool mediaFetchUiProbe: Qt.application.arguments.indexOf("--media-fetch-probe") >= 0
    readonly property bool favoriteProbe: Qt.application.arguments.indexOf("--favorite-probe") >= 0
    readonly property bool favoriteUiProbe: Qt.application.arguments.indexOf("--favorite-ui-probe") >= 0
    readonly property bool collectionProbe: Qt.application.arguments.indexOf("--collection-probe") >= 0
    readonly property bool collectionUiProbe: Qt.application.arguments.indexOf("--collection-ui-probe") >= 0
    readonly property bool downloadUiProbe: Qt.application.arguments.indexOf("--download-ui-probe") >= 0
    readonly property bool downloadHistoryProbe: Qt.application.arguments.indexOf("--download-history-probe") >= 0
    readonly property bool settingsUiProbe: Qt.application.arguments.indexOf("--settings-ui-probe") >= 0
    readonly property bool settingsSeedingProbe: Qt.application.arguments.indexOf("--settings-seeding-probe") >= 0
    readonly property bool settingsReleaseProbe: Qt.application.arguments.indexOf("--settings-release-probe") >= 0
    readonly property bool settingsRegionUiProbe: Qt.application.arguments.indexOf("--settings-region-ui-probe") >= 0
    readonly property bool controllerUiProbe: Qt.application.arguments.indexOf("--controller-ui-probe") >= 0
    readonly property bool controllerProfileUiProbe: Qt.application.arguments.indexOf("--controller-profile-ui-probe") >= 0
    readonly property bool alternateTitleUiProbe: Qt.application.arguments.indexOf("--alternate-title-ui-probe") >= 0
    readonly property bool releaseCandidateUiProbe: Qt.application.arguments.indexOf("--release-candidate-ui-probe") >= 0
    readonly property bool multidiscUiProbe: Qt.application.arguments.indexOf("--multidisc-ui-probe") >= 0
    readonly property bool exoArchiveUiProbe: Qt.application.arguments.indexOf("--exo-archive-ui-probe") >= 0
    readonly property bool laserdiscUiProbe: Qt.application.arguments.indexOf("--laserdisc-ui-probe") >= 0
    readonly property bool exoPrepareProbe: Qt.application.arguments.indexOf("--exo-prepare-probe") >= 0
    readonly property bool exoPrepareUiProbe: Qt.application.arguments.indexOf("--exo-prepare-ui-probe") >= 0
    readonly property bool exoLaunchProbe: Qt.application.arguments.indexOf("--exo-launch-probe") >= 0
    readonly property bool romLaunchProbe: Qt.application.arguments.indexOf("--rom-launch-probe") >= 0
    readonly property bool romLaunchUiProbe: Qt.application.arguments.indexOf("--rom-launch-ui-probe") >= 0
    readonly property bool launchProfileUiProbe: Qt.application.arguments.indexOf("--launch-profile-ui-probe") >= 0
    readonly property bool launchProfileManagerUiProbe: Qt.application.arguments.indexOf("--launch-profile-manager-ui-probe") >= 0
    readonly property bool arcadeLaunchProbe: Qt.application.arguments.indexOf("--arcade-launch-probe") >= 0
    readonly property bool arcadeLaunchUiProbe: Qt.application.arguments.indexOf("--arcade-launch-ui-probe") >= 0
    readonly property bool emulatorManagerProbe: Qt.application.arguments.indexOf("--emulator-manager-probe") >= 0
    readonly property bool emulatorManagerUiProbe: Qt.application.arguments.indexOf("--emulator-manager-ui-probe") >= 0
    readonly property bool emulatorUpdateUiProbe: Qt.application.arguments.indexOf("--emulator-update-ui-probe") >= 0
    readonly property bool firmwareProbe: Qt.application.arguments.indexOf("--firmware-probe") >= 0
    readonly property bool firmwareUiProbe: Qt.application.arguments.indexOf("--firmware-ui-probe") >= 0
    readonly property bool activityUiProbe: Qt.application.arguments.indexOf("--activity-ui-probe") >= 0
    readonly property bool mediaBundleProbe: Qt.application.arguments.indexOf("--media-bundle-probe") >= 0
    readonly property bool mediaBundleUiProbe: Qt.application.arguments.indexOf("--media-bundle-ui-probe") >= 0
    readonly property bool manualDownloadUiProbe: Qt.application.arguments.indexOf("--manual-download-ui-probe") >= 0
    readonly property bool box3dUiProbe: Qt.application.arguments.indexOf("--box-3d-ui-probe") >= 0
    readonly property bool mediaRotationUiProbe: Qt.application.arguments.indexOf("--media-rotation-ui-probe") >= 0
    readonly property bool emulatorLaunchProbe: exoLaunchProbe || romLaunchProbe || arcadeLaunchProbe
    readonly property bool downloadPlanUiProbe: multidiscUiProbe || exoArchiveUiProbe || laserdiscUiProbe
    readonly property string screenshotOutput: root.argumentValue("--screenshot-output")

    palette.window: "#0c1119"
    palette.windowText: ink
    palette.text: ink
    palette.buttonText: ink
    palette.highlight: accent

    onClosing: close => {
        if (library.favorite_pending_count > 0 || library.collection_busy) {
            close.accepted = false
            closeAfterFavoriteSave = true
        }
    }

    function scheduleFilter() {
        filterDelay.restart()
    }

    function argumentValue(name) {
        const applicationArguments = Qt.application.arguments
        const prefix = name + "="
        for (let index = 0; index < applicationArguments.length; ++index) {
            const value = String(applicationArguments[index])
            if (value === name && index + 1 < applicationArguments.length)
                return String(applicationArguments[index + 1])
            if (value.indexOf(prefix) === 0)
                return value.substring(prefix.length)
        }
        return ""
    }

    function finishSettingsProbeWhenSaved() {
        if ((root.settingsSeedingProbe && root.settingsSeedingTriggered
                || root.settingsReleaseProbe && root.settingsReleaseTriggered)
                && !appSettings.busy
                && appSettings.message.indexOf("Settings saved.") === 0)
            Qt.quit()
    }

    function openEmulatorManager() {
        emulatorManagerDialog.open()
        emulatorManager.initialize()
    }

    function openEmulatorUpdates() {
        emulatorUpdateDialog.open()
        emulatorUpdates.initialize()
    }

    function openLaunchProfileManager() {
        launchProfileManagerDialog.open()
        launchProfileManager.initialize()
    }

    function selectLibrary(availabilityKey) {
        selectedPlatform = ""
        selectedCollectionId = ""
        selectedCollectionName = ""
        availability = availabilityKey
        scheduleFilter()
    }

    function selectCollection(collectionId, collectionName) {
        selectedPlatform = ""
        selectedCollectionId = collectionId
        selectedCollectionName = collectionName
        availability = "collection:" + collectionId
        scheduleFilter()
    }

    function openNewCollectionDialog() {
        editingCollectionId = ""
        collectionNameField.text = ""
        collectionDescriptionField.text = ""
        collectionEditorDialog.open()
        collectionNameField.forceActiveFocus()
    }

    function openEditCollectionDialog(index) {
        editingCollectionId = library.collection_id_at(index)
        collectionNameField.text = library.collection_name_at(index)
        collectionDescriptionField.text = library.collection_description_at(index)
        collectionEditorDialog.open()
        collectionNameField.forceActiveFocus()
    }

    function saveEditedCollection() {
        if (collectionNameField.text.trim().length === 0 || library.collection_busy)
            return
        if (editingCollectionId.length > 0)
            library.update_collection(editingCollectionId,
                                      collectionNameField.text,
                                      collectionDescriptionField.text)
        else
            library.create_collection(collectionNameField.text,
                                      collectionDescriptionField.text)
        collectionEditorDialog.close()
    }

    function collectionIndex(collectionId) {
        for (let index = 0; index < library.collection_count; ++index) {
            if (library.collection_id_at(index) === collectionId)
                return index
        }
        return -1
    }

    function confirmRemoveDownload(index) {
        clearAllDownloadHistory = false
        pendingHistoryJobId = downloadQueue.job_id_at(index)
        pendingHistoryJobTitle = downloadQueue.job_title_at(index)
        downloadHistoryDialog.open()
    }

    function confirmClearDownloadHistory() {
        clearAllDownloadHistory = true
        pendingHistoryJobId = ""
        pendingHistoryJobTitle = ""
        downloadHistoryDialog.open()
    }

    function startCollectionProbeSearch() {
        collectionProbeStage = 2
        searchField.text = "A Nightmare on Elm Street"
        library.apply_filter(searchField.text, "", "")
    }

    function platformHeading() {
        if (selectedCollectionId.length > 0)
            return selectedCollectionName
        if (selectedPlatform.length > 0)
            return selectedPlatform
        if (availability === "local")
            return "My Collection"
        if (availability === "downloadable")
            return "Minerva Downloads"
        if (availability === "favorites")
            return "Favorites"
        if (availability === "recent")
            return "Recently Played"
        return "All Games"
    }

    function completionIndex(key) {
        for (let index = 0; index < completionChoices.length; ++index) {
            if (completionChoices[index].key === key)
                return index
        }
        return 0
    }

    function accentFor(value) {
        const colors = ["#f4775f", "#5f9df7", "#9b7cf7", "#4bc29b", "#e39f45", "#e26fa0"]
        if (!value || value.length === 0)
            return colors[0]
        return colors[value.charCodeAt(0) % colors.length]
    }

    function openGame(gameId, databaseId, title, platform, local, downloadable) {
        selectedGameId = gameId
        selectedDatabaseId = databaseId
        selectedBox3d = false
        selectedHeroArtworkIndex = 0
        library.request_artwork(databaseId, title, platform, library.artwork_type)
        library.request_artwork(databaseId, title, platform, "fanart")
        library.request_artwork(databaseId, title, platform, "box-front")
        refreshSelectedArtwork()
        gameDetails.select_game(gameId, title, platform, local, downloadable)
    }

    function refreshSelectedArtwork() {
        selectedArtworkUrl = library.artwork_url(selectedDatabaseId,
                                                 library.artwork_type)
        selectedHeroArtworkType = "fanart"
        selectedHeroArtworkCount = library.artwork_candidate_count(selectedDatabaseId,
                                                                    selectedHeroArtworkType)
        selectedHeroArtworkIndex = Math.max(0, Math.min(selectedHeroArtworkIndex,
                                                        selectedHeroArtworkCount - 1))
        selectedFanartUrl = selectedHeroArtworkCount > 0
                          ? library.artwork_candidate_url(selectedDatabaseId,
                                                          selectedHeroArtworkType,
                                                          selectedHeroArtworkIndex)
                          : ""
        selectedArtworkSource = selectedHeroArtworkCount > 0
                                ? library.artwork_candidate_source(selectedDatabaseId,
                                                                   selectedHeroArtworkType,
                                                                   selectedHeroArtworkIndex)
                                : library.artwork_source(selectedDatabaseId,
                                                         library.artwork_type)
        selectedBoxFrontUrl = library.exact_artwork_url(selectedDatabaseId,
                                                        "box-front")
        selectedBoxBackUrl = library.exact_artwork_url(selectedDatabaseId,
                                                       "box-back")
        if (selectedBoxFrontUrl.toString().length === 0)
            selectedBox3d = false
    }

    function activateBox3dProbeWhenReady() {
        if (!box3dUiProbe || selectedBoxFrontUrl.toString().length === 0)
            return
        selectedBox3d = true
        detailBox3d.resetView()
        detailBox3d.autoRotate = false
        console.log("LUNCHBOX_BOX3D_READY front=" + selectedBoxFrontUrl
                    + " back="
                    + (selectedBoxBackUrl.toString().length > 0))
    }

    function rotateSelectedHeroArtwork(offset) {
        if (selectedHeroArtworkCount < 2)
            return
        selectedHeroArtworkIndex = (selectedHeroArtworkIndex + offset
                                    + selectedHeroArtworkCount)
                                   % selectedHeroArtworkCount
        refreshSelectedArtwork()
    }

    function refreshHeroArtworkFromProvider() {
        if (selectedDatabaseId <= 0 || !library.media_retrieval_enabled)
            return
        selectedFanartUrl = ""
        selectedArtworkSource = ""
        library.redownload_artwork(selectedDatabaseId, gameDetails.title,
                                   gameDetails.platform, "fanart")
    }

    function activateMediaRotationProbeWhenReady() {
        if (!mediaRotationUiProbe || selectedHeroArtworkCount < 2)
            return
        selectedHeroArtworkIndex = 1
        refreshSelectedArtwork()
        console.log("LUNCHBOX_MEDIA_ROTATION_READY count="
                    + selectedHeroArtworkCount + " index="
                    + selectedHeroArtworkIndex + " source="
                    + selectedArtworkSource)
    }

    function artworkLabel(key) {
        for (let index = 0; index < artworkChoices.length; ++index) {
            if (artworkChoices[index].key === key)
                return artworkChoices[index].label
        }
        return "Artwork"
    }

    function artworkIndex(key) {
        for (let index = 0; index < artworkChoices.length; ++index) {
            if (artworkChoices[index].key === key)
                return index
        }
        return 0
    }

    function wideArtwork(key) {
        return key === "screenshot" || key === "title-screen" || key === "fanart"
    }

    function formatMediaTime(milliseconds) {
        const totalSeconds = Math.max(0, Math.floor(milliseconds / 1000))
        const hours = Math.floor(totalSeconds / 3600)
        const minutes = Math.floor((totalSeconds % 3600) / 60)
        const seconds = totalSeconds % 60
        if (hours > 0)
            return hours + ":" + String(minutes).padStart(2, "0")
                    + ":" + String(seconds).padStart(2, "0")
        return minutes + ":" + String(seconds).padStart(2, "0")
    }

    function openImportDialog() {
        importDialogLoader.active = true
        importDialogLoader.item.open()
    }

    LibraryModel {
        id: library
    }

    GameDetailsModel {
        id: gameDetails
    }

    AudioOutput {
        id: gameVideoAudio
        muted: true
        volume: 0.45
    }

    MediaPlayer {
        id: gameVideoPlayer
        property bool resumeApplied: false
        source: gameDetails.video_available ? gameDetails.video_url : ""
        audioOutput: gameVideoAudio
        videoOutput: mediaFullscreen.opened ? fullscreenVideoOutput : detailVideoOutput
        loops: MediaPlayer.Infinite
        onSourceChanged: {
            resumeApplied = false
            root.mediaPlaybackMessage = ""
            gameVideoAudio.muted = true
            if (source.toString().length === 0)
                stop()
        }
        onMediaStatusChanged: {
            if (mediaStatus === MediaPlayer.LoadedMedia) {
                if (!resumeApplied && seekable
                        && gameDetails.video_resume_position > 0)
                    position = Math.min(gameDetails.video_resume_position, duration)
                resumeApplied = true
                play()
            }
        }
        onErrorOccurred: function(error, errorString) {
            root.mediaPlaybackMessage = "Video playback failed: " + errorString
        }
        onPlaybackStateChanged: {
            if (playbackState === MediaPlayer.PausedState
                    && !gameDetails.video_progress_busy)
                gameDetails.save_video_progress(position, duration)
            if (root.mediaBundleProbe
                    && playbackState === MediaPlayer.PlayingState
                    && root.mediaBundleProbeStage === 0) {
                root.mediaBundleProbeStage = 1
                mediaProbePersistTimer.restart()
            }
        }
    }

    Timer {
        interval: 5000
        repeat: true
        running: gameVideoPlayer.playbackState === MediaPlayer.PlayingState
                 && gameDetails.video_available
        onTriggered: {
            if (!gameDetails.video_progress_busy)
                gameDetails.save_video_progress(gameVideoPlayer.position,
                                                gameVideoPlayer.duration)
        }
    }

    Timer {
        id: mediaProbePersistTimer
        interval: 800
        repeat: false
        onTriggered: {
            if (!gameDetails.manual_available) {
                console.error("LUNCHBOX_MEDIA_FAILED manual fixture missing")
                Qt.exit(2)
                return
            }
            gameVideoPlayer.position = Math.min(12000, gameVideoPlayer.duration)
            root.mediaBundleProbeStage = 2
            gameDetails.save_video_progress(gameVideoPlayer.position,
                                            gameVideoPlayer.duration)
        }
    }

    SettingsModel {
        id: appSettings
    }

    EmulatorManagerModel {
        id: emulatorManager
    }

    EmulatorUpdateModel {
        id: emulatorUpdates
    }

    LaunchProfileManagerModel {
        id: launchProfileManager
    }

    Connections {
        target: emulatorManager
        function onInitializedChanged() {
            if (root.emulatorManagerProbe && emulatorManager.initialized)
                Qt.quit()
        }
    }

    Connections {
        target: emulatorUpdates
        function onInitializedChanged() {
            if (root.emulatorUpdateUiProbe && emulatorUpdates.initialized)
                emulatorUpdateScreenshotTimer.restart()
        }
    }

    Connections {
        target: launchProfileManager
        function onInitializedChanged() {
            if (!root.launchProfileManagerUiProbe
                    || !launchProfileManager.initialized)
                return
            launchProfileSearch.text = "mame_rompath"
            launchProfileScopeFilter.currentIndex = 2
            launchProfileManager.apply_filter("mame_rompath", "platform", "all")
            if (launchProfileManager.row_count > 0)
                launchProfileManager.edit_at(0)
            launchProfileManagerScreenshotTimer.restart()
        }
        function onEditor_revisionChanged() {
            launchProfileManagerExtraArguments.text =
                    launchProfileManager.editor_extra_arguments
            launchProfileManagerCommandTemplate.text =
                    launchProfileManager.editor_command_template
        }
    }

    Connections {
        target: appSettings
        function onInitializedChanged() {
            if (root.settingsSeedingProbe && appSettings.initialized
                    && !root.settingsSeedingTriggered) {
                root.settingsSeedingTriggered = true
                appSettings.seeding_policy = "pause_after_import"
                appSettings.save()
            }
            if (root.settingsReleaseProbe && appSettings.initialized
                    && !root.settingsReleaseTriggered) {
                root.settingsReleaseTriggered = true
                for (let index = 0; index < appSettings.region_count(); ++index) {
                    if (appSettings.region_at(index) === "Japan") {
                        appSettings.move_region(index, 0)
                        break
                    }
                }
                appSettings.version_preference = "original"
                appSettings.save()
            }
        }
        function onBusyChanged() {
            root.finishSettingsProbeWhenSaved()
        }
        function onMessageChanged() {
            root.finishSettingsProbeWhenSaved()
        }
    }

    DownloadQueueModel {
        id: downloadQueue
    }

    LocalImportModel {
        id: localImport
    }

    Connections {
        target: library
        function onReadyChanged() {
            if (library.ready) {
                if (library.catalog_probe)
                    Qt.quit()
                else if (library.filter_probe)
                    library.apply_filter("mario", "", "downloadable")
                else if (root.collectionProbe || root.collectionUiProbe) {
                    if (library.collection_count === 0) {
                        root.collectionProbeStage = 1
                        library.create_collection("Collection Probe",
                                                  "Verifies native collection persistence and filtering")
                    } else {
                        root.startCollectionProbeSearch()
                    }
                }
                else if (root.favoriteProbe || root.favoriteUiProbe) {
                    searchField.text = "A Nightmare on Elm Street"
                    library.apply_filter(searchField.text, "", "")
                }
                else if (root.firmwareProbe || root.firmwareUiProbe) {
                    root.openGame("local-file:firmware-ui-probe", 0,
                                  "Buck Rogers: Planet of Zoom", "Coleco ADAM",
                                  true, false)
                }
                else if (root.activityUiProbe) {
                    root.selectLibrary("recent")
                }
                else {
                    root.scheduleFilter()
                    if (root.filterUiProbe)
                        filterPopup.open()
                    else if (root.downloadUiProbe || root.downloadHistoryProbe)
                        downloadsDrawer.open()
                }
            }
        }
        function onFilteringChanged() {
            if (library.filter_probe && library.ready && !library.filtering)
                Qt.quit()
            else if ((root.favoriteProbe || root.favoriteUiProbe)
                     && library.ready && !library.filtering
                     && searchField.text.length > 0)
                root.favoriteProbeArmed = true
            else if ((root.collectionProbe || root.collectionUiProbe)
                     && root.collectionProbeStage === 2
                     && library.ready && !library.filtering)
                root.collectionProbeArmed = true
            else if ((root.collectionProbe || root.collectionUiProbe)
                     && root.collectionProbeStage === 4
                     && library.ready && !library.filtering) {
                root.collectionProbeStage = 5
                if (root.collectionProbe)
                    Qt.quit()
                else
                    manageCollectionsDialog.open()
            }
        }
        function onMedia_revisionChanged() {
            if (library.media_probe)
                Qt.quit()
            else if (root.mediaFetchUiProbe && library.media_revision === 1) {
                searchField.text = "A Nightmare on Elm Street"
                library.apply_filter(searchField.text, "", "")
            }
            else if (root.artworkUiProbe && library.media_revision === 1) {
                searchField.text = "A Nightmare on Elm Street"
                library.apply_filter(searchField.text, "", "")
                viewPopup.open()
            } else if (root.mediaRotationUiProbe && !root.mediaRotationProbeOpened) {
                root.mediaRotationProbeOpened = true
                root.openGame("9697a5eb-e0b4-4f24-8d43-672701414ee7", 140,
                              "Super Mario Bros.",
                              "Nintendo Entertainment System", false, true)
                root.activateMediaRotationProbeWhenReady()
            } else if (root.box3dUiProbe && !root.box3dProbeOpened) {
                root.box3dProbeOpened = true
                root.openGame("9697a5eb-e0b4-4f24-8d43-672701414ee7", 140,
                              "Super Mario Bros.",
                              "Nintendo Entertainment System", false, true)
                root.activateBox3dProbeWhenReady()
            } else if (root.selectedGameId.length > 0) {
                root.refreshSelectedArtwork()
                root.activateBox3dProbeWhenReady()
                root.activateMediaRotationProbeWhenReady()
            }
        }
        function onArtwork_typeChanged() {
            if (root.selectedGameId.length > 0)
                root.refreshSelectedArtwork()
        }
        function onMedia_downloaded_countChanged() {
            if (root.mediaFetchUiProbe && library.media_downloaded_count > 0)
                Qt.quit()
        }
        function onFavorite_revisionChanged() {
            if (library.ready)
                root.scheduleFilter()
        }
        function onActivity_revisionChanged() {
            if (library.ready && root.availability === "recent")
                root.scheduleFilter()
        }
        function onFavorite_pending_countChanged() {
            if (root.closeAfterFavoriteSave
                    && library.favorite_pending_count === 0
                    && !library.collection_busy) {
                root.closeAfterFavoriteSave = false
                root.close()
            } else if (root.favoriteProbe && root.favoriteProbeTriggered
                    && library.favorite_pending_count === 0
                    && library.favorite_count > 0)
                Qt.quit()
        }
        function onCollection_revisionChanged() {
            if (root.selectedCollectionId.length > 0
                    && !library.collection_exists(root.selectedCollectionId))
                root.selectLibrary("")
            else if (root.selectedCollectionId.length > 0) {
                const selectedIndex = root.collectionIndex(root.selectedCollectionId)
                if (selectedIndex >= 0)
                    root.selectedCollectionName = library.collection_name_at(selectedIndex)
                root.scheduleFilter()
            }

            if ((root.collectionProbe || root.collectionUiProbe)
                    && root.collectionProbeStage === 1
                    && library.collection_count > 0
                    && !library.collection_busy)
                root.startCollectionProbeSearch()
        }
        function onCollection_busyChanged() {
            if (root.closeAfterFavoriteSave
                    && library.favorite_pending_count === 0
                    && !library.collection_busy) {
                root.closeAfterFavoriteSave = false
                root.close()
            }
            if ((root.collectionProbe || root.collectionUiProbe)
                    && root.collectionProbeStage === 1
                    && !library.collection_busy
                    && library.collection_count > 0)
                root.startCollectionProbeSearch()
            else if ((root.collectionProbe || root.collectionUiProbe)
                    && root.collectionProbeStage === 3
                    && !library.collection_busy
                    && library.collection_game_count_at(0) > 0) {
                root.collectionProbeStage = 4
                root.selectCollection(library.collection_id_at(0),
                                      library.collection_name_at(0))
            }
        }
    }

    Connections {
        target: gameDetails
        function onLaunch_profile_revisionChanged() {
            launchExtraArguments.text = gameDetails.launch_profile_extra_arguments
            launchCommandTemplate.text = gameDetails.launch_profile_command_template
            if (root.launchProfileUiProbe && gameDetails.launch_profile_open) {
                launchProfileProbeScrollTimer.restart()
                console.log("LUNCHBOX_LAUNCH_PROFILE_READY scope="
                            + gameDetails.launch_profile_scope + " template="
                            + gameDetails.launch_profile_default_template)
            }
        }
        function onActivity_revisionChanged() {
            library.refresh_activity()
            if (root.emulatorLaunchProbe && root.launchProbeAwaitingActivity
                    && !gameDetails.game_running)
                Qt.quit()
        }
        function onDownload_busyChanged() {
            if (!gameDetails.download_busy)
                downloadQueue.refresh()
        }
        function onBundle_countChanged() {
            if (root.releaseCandidateUiProbe && gameDetails.bundle_count > 0
                    && gameDetails.selected_bundle < 0)
                gameDetails.load_bundle_files(0)
            if (root.downloadPlanUiProbe && gameDetails.bundle_count > 0
                    && gameDetails.selected_bundle < 0) {
                if (root.laserdiscUiProbe) {
                    for (let i = 0; i < gameDetails.bundle_count; ++i) {
                        if (gameDetails.bundle_title_at(i).indexOf(
                                    "Laserdisc Collection · MAME") === 0) {
                            root.downloadPlanProbeBundleIndex = i
                            gameDetails.load_bundle_files(i)
                            return
                        }
                    }
                }
                root.downloadPlanProbeBundleIndex = 0
                gameDetails.load_bundle_files(0)
            }
        }
        function onAlternate_title_countChanged() {
            if (root.alternateTitleUiProbe
                    && gameDetails.alternate_title_count > 0) {
                console.log("LUNCHBOX_ALTERNATE_TITLES_READY count="
                            + gameDetails.alternate_title_count)
                alternateTitleProbeScrollTimer.restart()
            }
        }
        function onFile_countChanged() {
            if (root.releaseCandidateUiProbe && gameDetails.file_count > 0)
                detailsProbeScrollTimer.restart()
            if (root.downloadPlanUiProbe && gameDetails.file_count > 0) {
                for (let i = 0; i < gameDetails.file_count; ++i) {
                    if (gameDetails.file_has_download_plan(i)) {
                        root.pendingDownloadPlanIndex = i
                        downloadPlanDialog.open()
                        return
                    }
                }
                if (root.downloadPlanProbeBundleIndex + 1 < gameDetails.bundle_count) {
                    ++root.downloadPlanProbeBundleIndex
                    gameDetails.load_bundle_files(root.downloadPlanProbeBundleIndex)
                }
            }
        }
        function onTorrent_loadingChanged() {
            if (root.downloadPlanUiProbe && !gameDetails.torrent_loading
                    && gameDetails.selected_bundle >= 0
                    && gameDetails.file_count === 0
                    && root.downloadPlanProbeBundleIndex + 1 < gameDetails.bundle_count) {
                ++root.downloadPlanProbeBundleIndex
                gameDetails.load_bundle_files(root.downloadPlanProbeBundleIndex)
            }
        }
        function onPreparableChanged() {
            if (root.exoPrepareProbe && gameDetails.preparable
                    && !gameDetails.prepared && !gameDetails.prepare_busy)
                gameDetails.prepare_game()
        }
        function onPreparedChanged() {
            if (root.exoPrepareProbe && gameDetails.prepared
                    && !gameDetails.prepare_busy)
                Qt.quit()
        }
        function onPrepare_busyChanged() {
            if (root.exoPrepareProbe && gameDetails.prepared
                    && !gameDetails.prepare_busy)
                Qt.quit()
        }
        function onCan_launchChanged() {
            if (root.launchProfileUiProbe && gameDetails.can_launch
                    && !root.launchProfileProbeTriggered) {
                root.launchProfileProbeTriggered = true
                gameDetails.open_launch_profile_editor()
            } else if (root.emulatorLaunchProbe && gameDetails.can_launch
                    && !root.launchProbeTriggered) {
                root.launchProbeTriggered = true
                gameDetails.launch_game()
            }
        }
        function onGame_runningChanged() {
            if (gameDetails.game_running && gameDetails.video_available) {
                if (!gameDetails.video_progress_busy)
                    gameDetails.save_video_progress(gameVideoPlayer.position,
                                                    gameVideoPlayer.duration)
                gameVideoPlayer.pause()
            }
            if (!root.emulatorLaunchProbe
                    || !root.launchProbeTriggered)
                return
            if (gameDetails.game_running)
                root.launchProbeObservedRunning = true
            else if (root.launchProbeObservedRunning)
                root.launchProbeAwaitingActivity = true
        }
        function onLaunch_statusChanged() {
            if (root.emulatorLaunchProbe
                    && root.launchProbeTriggered
                    && gameDetails.launch_status.indexOf("Could not launch") === 0)
                Qt.exit(2)
        }
        function onFirmware_rule_countChanged() {
            if (root.firmwareProbe && gameDetails.firmware_rule_count > 0)
                Qt.quit()
        }
        function onMedia_revisionChanged() {
            if (root.mediaBundleUiProbe && gameDetails.media_visible) {
                mediaProbeScrollTimer.restart()
            }
        }
        function onMedia_visibleChanged() {
            if (root.mediaBundleUiProbe && gameDetails.media_visible) {
                mediaProbeScrollTimer.restart()
            }
        }
        function onVideo_progress_busyChanged() {
            if (root.mediaBundleProbe && root.mediaBundleProbeStage === 2
                    && !gameDetails.video_progress_busy) {
                console.log("LUNCHBOX_MEDIA_READY video="
                            + gameDetails.video_available + " manual="
                            + gameDetails.manual_available + " resume_ms="
                            + gameDetails.video_resume_position)
                Qt.quit()
            }
        }
        function onPanel_openChanged() {
            if (!gameDetails.panel_open && gameDetails.video_available) {
                if (!gameDetails.video_progress_busy)
                    gameDetails.save_video_progress(gameVideoPlayer.position,
                                                    gameVideoPlayer.duration)
                gameVideoPlayer.pause()
                mediaFullscreen.close()
            }
        }
        function onVideo_availableChanged() {
            if (!gameDetails.video_available)
                gameVideoPlayer.stop()
        }
    }

    Connections {
        target: downloadQueue
        function onCollection_revisionChanged() {
            if (downloadQueue.collection_revision > 0)
                library.reload()
        }
        function onRevisionChanged() {
            if (!root.downloadHistoryProbe || downloadQueue.busy)
                return
            if (!root.downloadHistoryTriggered
                    && downloadQueue.finished_count > 0) {
                root.downloadHistoryTriggered = true
                downloadQueue.clear_finished()
            } else if (root.downloadHistoryTriggered
                    && downloadQueue.finished_count === 0) {
                Qt.quit()
            }
        }
    }

    Connections {
        target: localImport
        function onCollection_revisionChanged() {
            if (localImport.collection_revision > 0) {
                root.selectLibrary("local")
                library.reload()
            }
        }
        function onInitializedChanged() {
            if (localImport.initialized && root.importUiProbe
                    && localImport.directory.length > 0)
                localImport.start_scan("", true)
        }
        function onResult_countChanged() {
            if (root.archiveImportUiProbe && localImport.result_count > 0)
                archiveImportScreenshotTimer.restart()
            if (root.importCommitProbe && localImport.result_count > 0
                    && localImport.selected_count === 0) {
                localImport.select_visible("all")
                localImport.import_selected()
            }
        }
    }

    Timer {
        id: archiveImportScreenshotTimer
        interval: 350
        repeat: false
        onTriggered: {
            const status = localImport.result_status_at(0)
            const archiveDetail = localImport.result_archive_detail_at(0)
            if (status !== "EXACT" || archiveDetail.length === 0) {
                console.error("LUNCHBOX_ARCHIVE_IMPORT_UI_FAILED status="
                              + status + " detail=" + archiveDetail)
                Qt.exit(2)
                return
            }
            if (root.screenshotOutput.length === 0) {
                console.log("LUNCHBOX_ARCHIVE_IMPORT_UI_READY detail="
                            + archiveDetail)
                Qt.quit()
                return
            }
            importDialogLoader.item.contentItem.grabToImage(function(result) {
                if (!result.saveToFile(root.screenshotOutput)) {
                    console.error("LUNCHBOX_ARCHIVE_IMPORT_UI_FAILED screenshot="
                                  + root.screenshotOutput)
                    Qt.exit(2)
                    return
                }
                console.log("LUNCHBOX_ARCHIVE_IMPORT_UI_READY detail="
                            + archiveDetail + " screenshot="
                            + root.screenshotOutput)
                Qt.quit()
            })
        }
    }

    Timer {
        id: startupTimer
        interval: 0
        running: true
        repeat: false
        onTriggered: {
            library.shell_ready()
            if (library.startup_probe)
                Qt.quit()
            else if (root.importUiProbe)
                root.openImportDialog()
            else if (root.settingsUiProbe || root.settingsRegionUiProbe
                     || root.controllerUiProbe || root.controllerProfileUiProbe) {
                settingsDialog.open()
                if (root.controllerUiProbe)
                    console.log("LUNCHBOX_CONTROLLER_UI_OPENED")
            }
            else if (root.emulatorManagerProbe)
                emulatorManager.initialize()
            else if (root.emulatorUpdateUiProbe)
                root.openEmulatorUpdates()
            else if (root.launchProfileManagerUiProbe)
                root.openLaunchProfileManager()
            else if (root.alternateTitleUiProbe)
                root.openGame("8ec34bc8-73d5-4e4a-be7b-73ed58e0dc9a", 2361,
                              "Pokémon Silver Version",
                              "Nintendo Game Boy Color", false, true)
            else if (root.releaseCandidateUiProbe)
                root.openGame("52f67472-bddb-4e5b-951b-43364f996573", 1726,
                              "Super Mario Land", "Nintendo Game Boy", false, true)
            else if (root.multidiscUiProbe)
                root.openGame("6119074e-d813-446d-a7ff-556f75e52320", 525,
                              "Final Fantasy VII", "Sony Playstation", false, true)
            else if (root.exoArchiveUiProbe)
                root.openGame("0faf424e-bb45-4d5f-b88d-771936a35f8a", 14329,
                              "Prince of Persia", "MS-DOS", false, true)
            else if (root.laserdiscUiProbe)
                root.openGame("ffecdc12-bc0f-459c-a0b7-4ee856e14f9e", 33536,
                              "Dragon's Lair", "Arcade Laserdisc", false, true)
            else if (root.exoPrepareProbe || root.exoPrepareUiProbe
                     || root.exoLaunchProbe)
                root.openGame("exo-prepare-probe", 0,
                              "Prince of Persia", "MS-DOS", true, false)
            else if (root.arcadeLaunchProbe || root.arcadeLaunchUiProbe)
                root.openGame("local-file:arcade-launch-probe", 0,
                              "Pong", "Arcade", true, false)
            else if (root.romLaunchProbe || root.romLaunchUiProbe
                     || root.launchProfileUiProbe)
                root.openGame("local-file:rom-launch-probe", 0,
                              "Faxanadu", "Nintendo Entertainment System", true, false)
            else if (root.activityUiProbe)
                root.openGame("local-file:rom-launch-probe", 0,
                              "Faxanadu", "Nintendo Entertainment System", true, false)
            else if (root.mediaBundleProbe || root.mediaBundleUiProbe)
                root.openGame("9697a5eb-e0b4-4f24-8d43-672701414ee7", 140,
                              "Super Mario Bros.", "Nintendo Entertainment System",
                              false, true)
            else if (root.manualDownloadUiProbe)
                root.openGame("ffb0ddd4-d5e1-4f78-90c8-068db5022cd5", 0,
                              "Cooking Pico - Minna to Issho ni Hajimete Cooking! (Japan)",
                              "Sega - PICO", false, false)
        }
    }

    Timer {
        interval: 400
        running: root.emulatorManagerUiProbe
        repeat: false
        onTriggered: root.openEmulatorManager()
    }

    Timer {
        interval: 500
        running: root.settingsRegionUiProbe
        repeat: false
        onTriggered: settingsScroll.contentItem.contentY = Math.max(
                         0, regionPrioritySection.mapToItem(
                             settingsScroll.contentItem, 0, 0).y - 18)
    }

    Timer {
        interval: 800
        running: root.controllerUiProbe && !appSettings.controller_busy
        repeat: false
        onTriggered: settingsScroll.contentItem.contentY = Math.max(
                         0, controllerSection.mapToItem(
                             settingsScroll.contentItem, 0, 0).y - 18)
    }

    Timer {
        interval: 900
        running: root.controllerProfileUiProbe && appSettings.initialized
        repeat: false
        onTriggered: {
            appSettings.create_controller_profile()
            appSettings.apply_two_button_controller_preset()
            settingsScroll.contentItem.contentY = Math.max(
                0, controllerProfileEditor.mapToItem(
                    settingsScroll.contentItem, 0, 0).y - 18)
        }
    }

    Timer {
        id: catalogTimer
        interval: 0
        running: true
        repeat: false
        onTriggered: {
            library.initialize()
            appSettings.initialize()
            downloadQueue.initialize()
            localImport.initialize()
        }
    }

    Timer {
        interval: 2000
        running: downloadQueue.active_count > 0 || downloadsDrawer.opened
        repeat: true
        onTriggered: downloadQueue.refresh()
    }

    Timer {
        interval: 2000
        running: gameDetails.manual_transfer_active
                 && !gameDetails.manual_action_busy
        repeat: true
        onTriggered: gameDetails.refresh_manual_download()
    }

    Timer {
        id: filterDelay
        interval: 75
        repeat: false
        onTriggered: library.apply_filter(searchField.text, root.selectedPlatform,
                                           root.availability)
    }

    Timer {
        id: detailsProbeScrollTimer
        interval: 100
        repeat: false
        onTriggered: detailScroll.contentItem.contentY = Math.max(
                         0, fileCandidateHeading.mapToItem(
                             detailScroll.contentItem, 0, 0).y - 12)
    }

    Timer {
        id: alternateTitleProbeScrollTimer
        interval: 100
        repeat: false
        onTriggered: {
            detailScroll.contentItem.contentY = Math.max(
                        0, alternateTitleSection.mapToItem(
                            detailScroll.contentItem, 0, 0).y - 12)
            alternateTitleScreenshotTimer.restart()
        }
    }

    Timer {
        id: alternateTitleScreenshotTimer
        interval: 350
        repeat: false
        onTriggered: {
            if (root.screenshotOutput.length === 0)
                return
            detailsPane.grabToImage(function(result) {
                if (!result.saveToFile(root.screenshotOutput)) {
                    console.error("LUNCHBOX_SCREENSHOT_FAILED path="
                                  + root.screenshotOutput)
                    Qt.exit(2)
                    return
                }
                console.log("LUNCHBOX_SCREENSHOT_READY path="
                            + root.screenshotOutput)
                Qt.quit()
            })
        }
    }

    Timer {
        id: mediaProbeScrollTimer
        interval: 700
        repeat: false
        onTriggered: detailScroll.contentItem.contentY = Math.max(
                         0, mediaCard.mapToItem(
                             detailScroll.contentItem, 0, 0).y - 12)
    }

    Timer {
        id: launchProfileProbeScrollTimer
        interval: 100
        repeat: false
        onTriggered: {
            detailScroll.contentItem.contentY = Math.max(
                        0, launchProfileCard.mapToItem(
                            detailScroll.contentItem, 0, 0).y - 8)
            launchProfileScreenshotTimer.restart()
        }
    }

    Timer {
        id: launchProfileScreenshotTimer
        interval: 350
        repeat: false
        onTriggered: {
            if (root.screenshotOutput.length === 0)
                return
            detailsPane.grabToImage(function(result) {
                if (!result.saveToFile(root.screenshotOutput)) {
                    console.error("LUNCHBOX_SCREENSHOT_FAILED path="
                                  + root.screenshotOutput)
                    Qt.exit(2)
                    return
                }
                console.log("LUNCHBOX_SCREENSHOT_READY path="
                            + root.screenshotOutput)
                Qt.quit()
            })
        }
    }

    Popup {
        id: mediaFullscreen
        parent: Overlay.overlay
        x: 0
        y: 0
        width: root.width
        height: root.height
        padding: 0
        modal: true
        focus: true
        closePolicy: Popup.CloseOnEscape
        background: Rectangle { color: "#f205080c" }
        onClosed: {
            if (!gameDetails.video_progress_busy)
                gameDetails.save_video_progress(gameVideoPlayer.position,
                                                gameVideoPlayer.duration)
        }

        VideoOutput {
            id: fullscreenVideoOutput
            anchors.fill: parent
            anchors.margins: 18
            fillMode: VideoOutput.PreserveAspectFit
        }

        MouseArea {
            anchors.fill: parent
            anchors.bottomMargin: fullscreenControls.height
            onClicked: {
                if (gameVideoPlayer.playbackState === MediaPlayer.PlayingState)
                    gameVideoPlayer.pause()
                else
                    gameVideoPlayer.play()
            }
        }

        RowLayout {
            id: fullscreenControls
            anchors.left: parent.left
            anchors.right: parent.right
            anchors.bottom: parent.bottom
            anchors.margins: 18
            height: 54
            spacing: 10

            Button {
                Layout.preferredWidth: 54
                Layout.preferredHeight: 38
                text: gameVideoPlayer.playbackState === MediaPlayer.PlayingState ? "Ⅱ" : "▶"
                onClicked: {
                    if (gameVideoPlayer.playbackState === MediaPlayer.PlayingState)
                        gameVideoPlayer.pause()
                    else
                        gameVideoPlayer.play()
                }
            }
            Slider {
                id: fullscreenPosition
                Layout.fillWidth: true
                from: 0
                to: Math.max(1, gameVideoPlayer.duration)
                onMoved: {
                    if (gameVideoPlayer.seekable)
                        gameVideoPlayer.position = value
                }
                Binding on value {
                    when: !fullscreenPosition.pressed
                    value: gameVideoPlayer.position
                }
            }
            Text {
                Layout.preferredWidth: 112
                text: root.formatMediaTime(gameVideoPlayer.position)
                      + " / " + root.formatMediaTime(gameVideoPlayer.duration)
                color: root.ink
                font.pixelSize: 11
                font.features: { "tnum": 1 }
                horizontalAlignment: Text.AlignHCenter
            }
            Button {
                Layout.preferredWidth: 72
                Layout.preferredHeight: 38
                text: gameVideoAudio.muted ? "UNMUTE" : "MUTE"
                onClicked: gameVideoAudio.muted = !gameVideoAudio.muted
            }
            Button {
                Layout.preferredWidth: 72
                Layout.preferredHeight: 38
                text: "CLOSE"
                onClicked: mediaFullscreen.close()
            }
        }
    }

    Popup {
        id: filterPopup
        parent: Overlay.overlay
        x: Math.max(sidebar.width + 18,
                    root.width - detailsPane.width - width - 28)
        y: header.height + 76
        width: 326
        padding: 10
        focus: true
        closePolicy: Popup.CloseOnEscape | Popup.CloseOnReleaseOutside
        implicitHeight: filterColumn.implicitHeight + topPadding + bottomPadding

        background: Rectangle {
            radius: 12
            color: "#111925"
            border.color: root.line
            border.width: 1
        }
        contentItem: Column {
            id: filterColumn
            spacing: 2
            Item {
                width: parent.width
                height: 40
                Text {
                    anchors.left: parent.left
                    anchors.verticalCenter: parent.verticalCenter
                    text: "LIBRARY FILTERS"
                    color: root.ink
                    font.pixelSize: 11
                    font.weight: Font.Bold
                    font.letterSpacing: 1
                }
                Button {
                    anchors.right: parent.right
                    anchors.verticalCenter: parent.verticalCenter
                    text: "Clear"
                    flat: true
                    enabled: root.activeFilterCount > 0
                    onClicked: {
                        library.set_content_filters(false, false)
                        root.selectLibrary("")
                    }
                }
            }
            FilterToggle {
                label: "Installed"
                description: "Games already present in My Collection"
                checked: root.availability === "local"
                onToggled: root.selectLibrary(checked ? "" : "local")
            }
            FilterToggle {
                label: "Downloadable from Minerva"
                description: "Not installed and covered by an exact platform offer"
                checked: root.availability === "downloadable"
                onToggled: root.selectLibrary(checked ? "" : "downloadable")
            }
            FilterToggle {
                label: "Favorites"
                description: "Games saved to your personal Favorites list"
                checked: root.availability === "favorites"
                onToggled: root.selectLibrary(checked ? "" : "favorites")
            }
            FilterToggle {
                label: "Recently played"
                description: "Games ordered by your latest play session"
                checked: root.availability === "recent"
                onToggled: root.selectLibrary(checked ? "" : "recent")
            }
            Rectangle {
                width: parent.width
                height: 1
                color: root.line
            }
            FilterToggle {
                label: "Hide non-retail"
                description: "Homebrew, hacks, unlicensed and pirate releases"
                checked: library.hide_non_retail
                onToggled: {
                    library.set_content_filters(!checked, library.hide_adult)
                    root.scheduleFilter()
                }
            }
            FilterToggle {
                label: "Hide adult"
                description: "Adult-only ratings, genres and explicit title tags"
                checked: library.hide_adult
                onToggled: {
                    library.set_content_filters(library.hide_non_retail, !checked)
                    root.scheduleFilter()
                }
            }
            Text {
                width: parent.width
                topPadding: 7
                bottomPadding: 3
                text: "Filters combine with search and platform selection. Minerva always excludes installed games."
                color: "#6f7c90"
                font.pixelSize: 9
                wrapMode: Text.WordWrap
            }
        }
    }

    Popup {
        id: viewPopup
        parent: Overlay.overlay
        x: Math.max(sidebar.width + 18,
                    root.width - detailsPane.width - width - 28)
        y: header.height + 76
        width: 334
        padding: 16
        focus: true
        closePolicy: Popup.CloseOnEscape | Popup.CloseOnReleaseOutside
        implicitHeight: viewColumn.implicitHeight + topPadding + bottomPadding
        onOpened: {
            artworkCombo.currentIndex = root.artworkIndex(library.artwork_type)
            zoomSlider.value = library.grid_zoom
        }

        background: Rectangle {
            radius: 12
            color: "#111925"
            border.color: root.line
            border.width: 1
        }

        contentItem: Column {
            id: viewColumn
            spacing: 11
            Text {
                text: "LIBRARY APPEARANCE"
                color: root.ink
                font.pixelSize: 11
                font.weight: Font.Bold
                font.letterSpacing: 1
            }
            Text {
                text: "ARTWORK"
                color: "#687488"
                font.pixelSize: 9
                font.weight: Font.Bold
                font.letterSpacing: 1
            }
            ComboBox {
                id: artworkCombo
                width: parent.width
                height: 40
                model: root.artworkChoices
                textRole: "label"
                currentIndex: root.artworkIndex(library.artwork_type)
                onActivated: library.set_presentation_preferences(
                                 root.artworkChoices[index].key,
                                 Math.round(zoomSlider.value))
            }
            Row {
                width: parent.width
                spacing: 8
                Text {
                    text: "CARD SIZE"
                    color: "#687488"
                    font.pixelSize: 9
                    font.weight: Font.Bold
                    font.letterSpacing: 1
                }
                Text {
                    width: parent.width - 76
                    text: Math.round(zoomSlider.value) + "%"
                    horizontalAlignment: Text.AlignRight
                    color: root.ink
                    font.pixelSize: 10
                    font.weight: Font.DemiBold
                }
            }
            Row {
                width: parent.width
                spacing: 9
                Button {
                    width: 35
                    height: 35
                    text: "−"
                    enabled: zoomSlider.value > zoomSlider.from
                    onClicked: {
                        const next = Math.max(zoomSlider.from, zoomSlider.value - 10)
                        zoomSlider.value = next
                        library.set_presentation_preferences(library.artwork_type,
                                                             Math.round(next))
                    }
                }
                Slider {
                    id: zoomSlider
                    width: parent.width - 88
                    anchors.verticalCenter: parent.verticalCenter
                    from: 50
                    to: 200
                    stepSize: 5
                    value: library.grid_zoom
                    snapMode: Slider.SnapAlways
                    onPressedChanged: {
                        if (!pressed && Math.round(value) !== library.grid_zoom)
                            library.set_presentation_preferences(
                                        library.artwork_type, Math.round(value))
                    }
                }
                Button {
                    width: 35
                    height: 35
                    text: "+"
                    enabled: zoomSlider.value < zoomSlider.to
                    onClicked: {
                        const next = Math.min(zoomSlider.to, zoomSlider.value + 10)
                        zoomSlider.value = next
                        library.set_presentation_preferences(library.artwork_type,
                                                             Math.round(next))
                    }
                }
            }
            Rectangle {
                width: parent.width
                height: 1
                color: root.line
            }
            Text {
                width: parent.width
                text: library.media_loading
                      ? "Indexing artwork without blocking the library…"
                      : library.media_asset_count > 0
                        ? library.media_asset_count + " preferred assets for "
                          + library.media_game_count + " games"
                        : "No local media cache found. Cards use a fast generated fallback."
                color: root.muted
                font.pixelSize: 10
                wrapMode: Text.WordWrap
            }
            Text {
                width: parent.width
                text: library.media_fetch_message
                color: library.media_error_count > 0 ? root.accent : root.accentCool
                font.pixelSize: 10
                wrapMode: Text.WordWrap
            }
            Text {
                width: parent.width
                visible: library.media_pending_count > 0
                         || library.media_downloaded_count > 0
                         || library.media_missing_count > 0
                text: library.media_pending_count + " queued · "
                      + library.media_downloaded_count + " downloaded · "
                      + library.media_missing_count + " unavailable"
                color: "#657186"
                font.pixelSize: 9
            }
            Text {
                width: parent.width
                visible: library.media_directory.length > 0
                text: library.media_directory
                color: "#657186"
                font.pixelSize: 9
                elide: Text.ElideMiddle
            }
        }
    }

    component HeaderButton: Button {
        id: control
        property bool active: false
        implicitHeight: 38
        leftPadding: 16
        rightPadding: 16
        font.pixelSize: 13
        font.weight: Font.DemiBold
        background: Rectangle {
            radius: 9
            color: control.down ? "#303b4d" : control.active ? "#34303a" : "#1b2330"
            border.color: control.active ? root.accent : root.line
            border.width: 1
        }
        contentItem: Text {
            text: control.text
            color: control.active ? root.accent : root.ink
            font: control.font
            verticalAlignment: Text.AlignVCenter
            horizontalAlignment: Text.AlignHCenter
        }
    }

    component NavButton: Rectangle {
        id: nav
        required property string label
        required property string glyph
        property string count: ""
        property bool active: false
        signal clicked()
        width: ListView.view ? ListView.view.width : 228
        height: 43
        radius: 9
        color: active ? "#272c34" : hover.hovered ? "#1b2330" : "transparent"
        border.color: active ? "#443b31" : "transparent"

        HoverHandler { id: hover }
        TapHandler { onTapped: nav.clicked() }

        Rectangle {
            visible: nav.active
            width: 3
            height: 23
            radius: 2
            color: root.accent
            anchors.left: parent.left
            anchors.leftMargin: 1
            anchors.verticalCenter: parent.verticalCenter
        }
        Text {
            text: nav.glyph
            width: 28
            anchors.left: parent.left
            anchors.leftMargin: 13
            anchors.verticalCenter: parent.verticalCenter
            color: nav.active ? root.accent : root.muted
            font.pixelSize: 16
            horizontalAlignment: Text.AlignHCenter
        }
        Text {
            text: nav.label
            anchors.left: parent.left
            anchors.leftMargin: 54
            anchors.right: countText.left
            anchors.rightMargin: 8
            anchors.verticalCenter: parent.verticalCenter
            elide: Text.ElideRight
            color: nav.active ? root.ink : "#c0c8d4"
            font.pixelSize: 14
            font.weight: nav.active ? Font.DemiBold : Font.Medium
        }
        Text {
            id: countText
            text: nav.count
            anchors.right: parent.right
            anchors.rightMargin: 13
            anchors.verticalCenter: parent.verticalCenter
            color: root.muted
            font.pixelSize: 12
            font.features: { "tnum": 1 }
        }
    }

    component StatusPill: Rectangle {
        id: pill
        required property string label
        required property string value
        implicitWidth: statusRow.implicitWidth + 24
        implicitHeight: 34
        radius: 9
        color: "#151d29"
        border.color: root.line
        Row {
            id: statusRow
            anchors.centerIn: parent
            spacing: 7
            Text {
                text: pill.value
                color: root.ink
                font.pixelSize: 13
                font.weight: Font.Bold
            }
            Text {
                text: pill.label
                color: root.muted
                font.pixelSize: 12
            }
        }
    }

    component FilterToggle: Rectangle {
        id: filterToggle
        required property string label
        required property string description
        property bool checked: false
        signal toggled()
        width: filterPopup.availableWidth
        height: 58
        radius: 8
        color: filterHover.hovered ? "#1b2432" : "transparent"

        HoverHandler { id: filterHover }
        TapHandler { onTapped: filterToggle.toggled() }

        Rectangle {
            anchors.left: parent.left
            anchors.leftMargin: 11
            anchors.verticalCenter: parent.verticalCenter
            width: 18
            height: 18
            radius: 5
            color: filterToggle.checked ? root.accentCool : "#101721"
            border.color: filterToggle.checked ? root.accentCool : "#455166"
            Text {
                anchors.centerIn: parent
                text: filterToggle.checked ? "✓" : ""
                color: "#0c1716"
                font.pixelSize: 13
                font.weight: Font.Black
            }
        }
        Column {
            anchors.left: parent.left
            anchors.leftMargin: 42
            anchors.right: parent.right
            anchors.rightMargin: 10
            anchors.verticalCenter: parent.verticalCenter
            spacing: 2
            Text {
                width: parent.width
                text: filterToggle.label
                color: root.ink
                font.pixelSize: 12
                font.weight: Font.DemiBold
                elide: Text.ElideRight
            }
            Text {
                width: parent.width
                text: filterToggle.description
                color: root.muted
                font.pixelSize: 9
                elide: Text.ElideRight
            }
        }
    }

    component GameGrid: GridView {
        id: grid
        reuseItems: true
        clip: true
        cacheBuffer: height
        keyNavigationEnabled: true
        activeFocusOnTab: true
        boundsBehavior: Flickable.StopAtBounds
        property real preferredWidth: (root.width >= 1500 ? 230 : 205)
                                      * library.grid_zoom / 100
        cellWidth: Math.floor(width / Math.max(1, Math.floor(width / preferredWidth)))
        cellHeight: Math.round(cellWidth * 1.36)
        model: library
        ScrollBar.vertical: ScrollBar { policy: ScrollBar.AsNeeded }

        delegate: Item {
            id: tile
            required property int index
            required property string gameId
            required property string gameTitle
            required property string gamePlatform
            required property bool gameLocal
            required property bool gameDownloadable
            required property int gameDatabaseId
            property int mediaRevision: library.media_revision
            property int favoriteRevision: library.favorite_revision
            property int favoritePendingRevision: library.favorite_pending_count
            property bool favorite: {
                favoriteRevision
                return library.is_favorite(gameId)
            }
            property bool favoriteBusy: {
                favoritePendingRevision
                return library.favorite_pending(gameId)
            }
            property bool favoriteProbeReady: root.favoriteProbeArmed
            property bool collectionProbeReady: root.collectionProbeArmed
            property string requestedArtworkType: library.artwork_type
            property url artworkUrl: {
                mediaRevision
                return library.artwork_url(gameDatabaseId, library.artwork_type)
            }
            property string artworkSource: {
                mediaRevision
                return library.artwork_source(gameDatabaseId, library.artwork_type)
            }
            function requestVisibleArtwork() {
                library.request_artwork(gameDatabaseId, gameTitle,
                                        gamePlatform, requestedArtworkType)
            }
            function runFavoriteProbe() {
                if (!favoriteProbeReady || index !== 0 || root.favoriteProbeTriggered)
                    return
                root.favoriteProbeTriggered = true
                root.openGame(gameId, gameDatabaseId, gameTitle, gamePlatform,
                              gameLocal, gameDownloadable)
                if (!favorite)
                    library.set_favorite(gameId, true)
            }
            function runCollectionProbe() {
                if (!collectionProbeReady || index !== 0
                        || root.collectionProbeStage !== 2)
                    return
                root.openGame(gameId, gameDatabaseId, gameTitle, gamePlatform,
                              gameLocal, gameDownloadable)
                if (library.collection_contains(library.collection_id_at(0), gameId)) {
                    root.collectionProbeStage = 4
                    root.selectCollection(library.collection_id_at(0),
                                          library.collection_name_at(0))
                    return
                }
                root.collectionProbeStage = 3
                library.set_collection_membership(library.collection_id_at(0),
                                                  gameId, true)
            }
            Component.onCompleted: {
                requestVisibleArtwork()
                runFavoriteProbe()
                runCollectionProbe()
            }
            onGameDatabaseIdChanged: requestVisibleArtwork()
            onGameTitleChanged: requestVisibleArtwork()
            onGamePlatformChanged: requestVisibleArtwork()
            onMediaRevisionChanged: requestVisibleArtwork()
            onRequestedArtworkTypeChanged: requestVisibleArtwork()
            onFavoriteProbeReadyChanged: runFavoriteProbe()
            onCollectionProbeReadyChanged: runCollectionProbe()
            width: grid.cellWidth
            height: grid.cellHeight
            activeFocusOnTab: true

            TapHandler {
                onTapped: {
                    tile.forceActiveFocus()
                    root.openGame(tile.gameId, tile.gameDatabaseId,
                                  tile.gameTitle, tile.gamePlatform,
                                  tile.gameLocal, tile.gameDownloadable)
                }
            }
            Keys.onPressed: event => {
                if (event.key === Qt.Key_Return || event.key === Qt.Key_Enter
                        || event.key === Qt.Key_Space) {
                    root.openGame(tile.gameId, tile.gameDatabaseId,
                                  tile.gameTitle, tile.gamePlatform,
                                  tile.gameLocal, tile.gameDownloadable)
                    event.accepted = true
                } else if (event.key === Qt.Key_F && !tile.favoriteBusy) {
                    library.set_favorite(tile.gameId, !tile.favorite)
                    event.accepted = true
                }
            }

            Rectangle {
                id: card
                anchors.fill: parent
                anchors.margins: 8
                radius: 12
                color: tile.activeFocus ? "#263142" : cardHover.hovered ? "#202a39" : root.panelRaised
                border.color: tile.activeFocus || root.selectedGameId === tile.gameId ? root.accent : cardHover.hovered ? "#3a485d" : root.line
                border.width: tile.activeFocus || root.selectedGameId === tile.gameId ? 2 : 1
                scale: cardHover.hovered ? 1.012 : 1
                Behavior on scale { NumberAnimation { duration: 90 } }

                Rectangle {
                    id: artwork
                    anchors.left: parent.left
                    anchors.right: parent.right
                    anchors.top: parent.top
                    anchors.margins: 9
                    height: parent.height - 79
                    radius: 9
                    color: root.accentFor(tile.gameTitle)
                    clip: true

                    gradient: Gradient {
                        GradientStop { position: 0; color: Qt.lighter(artwork.color, 1.17) }
                        GradientStop { position: 1; color: Qt.darker(artwork.color, 1.55) }
                    }

                    Image {
                        id: coverImage
                        anchors.fill: parent
                        anchors.margins: 1
                        source: tile.artworkUrl
                        asynchronous: true
                        cache: true
                        mipmap: true
                        autoTransform: true
                        fillMode: root.wideArtwork(library.artwork_type)
                                  ? Image.PreserveAspectCrop : Image.PreserveAspectFit
                        sourceSize.width: Math.max(1, Math.round(width * 2))
                        sourceSize.height: Math.max(1, Math.round(height * 2))
                        opacity: status === Image.Ready ? 1 : 0
                        Behavior on opacity { NumberAnimation { duration: 130 } }
                    }

                    Text {
                        anchors.centerIn: parent
                        width: parent.width - 28
                        text: tile.gameTitle.length > 0 ? tile.gameTitle.charAt(0).toUpperCase() : "?"
                        horizontalAlignment: Text.AlignHCenter
                        color: "#ddffffff"
                        font.pixelSize: Math.min(72, parent.height * 0.38)
                        font.weight: Font.Black
                        visible: coverImage.status !== Image.Ready
                    }
                    Rectangle {
                        visible: tile.gameLocal || tile.gameDownloadable
                        anchors.top: parent.top
                        anchors.right: parent.right
                        anchors.margins: 9
                        implicitWidth: badgeText.implicitWidth + 14
                        implicitHeight: 25
                        radius: 7
                        color: tile.gameLocal ? "#d91d3d35" : "#d9303540"
                        border.color: tile.gameLocal ? root.accentCool : root.accent
                        Text {
                            id: badgeText
                            anchors.centerIn: parent
                            text: tile.gameLocal ? "READY" : "GET"
                            color: tile.gameLocal ? root.accentCool : root.accent
                            font.pixelSize: 10
                            font.weight: Font.Bold
                            font.letterSpacing: 0.8
                        }
                    }
                    RoundButton {
                        anchors.left: parent.left
                        anchors.top: parent.top
                        anchors.margins: 8
                        width: 32
                        height: 32
                        visible: tile.favorite || tile.favoriteBusy
                                 || cardHover.hovered || tile.activeFocus
                        enabled: !tile.favoriteBusy
                        text: tile.favoriteBusy ? "…" : tile.favorite ? "★" : "☆"
                        flat: true
                        font.pixelSize: 17
                        onClicked: library.set_favorite(tile.gameId, !tile.favorite)
                        background: Rectangle {
                            radius: 9
                            color: "#d9101620"
                            border.color: tile.favorite ? root.accent : "#5b687d"
                        }
                        contentItem: Text {
                            text: parent.text
                            color: tile.favorite ? root.accent : root.ink
                            font: parent.font
                            horizontalAlignment: Text.AlignHCenter
                            verticalAlignment: Text.AlignVCenter
                        }
                        ToolTip.visible: hovered
                        ToolTip.text: tile.favorite ? "Remove from Favorites" : "Add to Favorites"
                    }
                }
                ToolTip.visible: cardHover.hovered && tile.artworkSource.length > 0
                ToolTip.text: root.artworkLabel(library.artwork_type)
                              + " · " + tile.artworkSource

                Text {
                    anchors.left: parent.left
                    anchors.right: parent.right
                    anchors.leftMargin: 12
                    anchors.rightMargin: 12
                    anchors.top: artwork.bottom
                    anchors.topMargin: 10
                    text: tile.gameTitle
                    color: root.ink
                    font.pixelSize: 14
                    font.weight: Font.DemiBold
                    elide: Text.ElideRight
                }
                Text {
                    anchors.left: parent.left
                    anchors.right: parent.right
                    anchors.leftMargin: 12
                    anchors.rightMargin: 12
                    anchors.bottom: parent.bottom
                    anchors.bottomMargin: 9
                    text: tile.gamePlatform.length > 0 ? tile.gamePlatform : "Unassigned platform"
                    color: root.muted
                    font.pixelSize: 11
                    elide: Text.ElideRight
                }
            }
            HoverHandler { id: cardHover }
        }
    }

    component GameList: ListView {
        id: list
        reuseItems: true
        clip: true
        cacheBuffer: height
        spacing: 6
        model: library
        boundsBehavior: Flickable.StopAtBounds
        ScrollBar.vertical: ScrollBar { policy: ScrollBar.AsNeeded }
        delegate: Rectangle {
            id: row
            required property int index
            required property string gameId
            required property string gameTitle
            required property string gamePlatform
            required property string gameStatus
            required property bool gameLocal
            required property bool gameDownloadable
            required property int gameDatabaseId
            property int mediaRevision: library.media_revision
            property int favoriteRevision: library.favorite_revision
            property int favoritePendingRevision: library.favorite_pending_count
            property bool favorite: {
                favoriteRevision
                return library.is_favorite(gameId)
            }
            property bool favoriteBusy: {
                favoritePendingRevision
                return library.favorite_pending(gameId)
            }
            property string requestedArtworkType: library.artwork_type
            property url artworkUrl: {
                mediaRevision
                return library.artwork_url(gameDatabaseId, library.artwork_type)
            }
            function requestVisibleArtwork() {
                library.request_artwork(gameDatabaseId, gameTitle,
                                        gamePlatform, requestedArtworkType)
            }
            Component.onCompleted: requestVisibleArtwork()
            onGameDatabaseIdChanged: requestVisibleArtwork()
            onGameTitleChanged: requestVisibleArtwork()
            onGamePlatformChanged: requestVisibleArtwork()
            onMediaRevisionChanged: requestVisibleArtwork()
            onRequestedArtworkTypeChanged: requestVisibleArtwork()
            width: list.width - 8
            height: 62
            radius: 9
            activeFocusOnTab: true
            color: row.activeFocus ? "#263142" : rowHover.hovered ? "#202a39" : root.panelRaised
            border.color: row.activeFocus || root.selectedGameId === row.gameId ? root.accent : rowHover.hovered ? "#3a485d" : root.line
            border.width: row.activeFocus || root.selectedGameId === row.gameId ? 2 : 1
            HoverHandler { id: rowHover }
            TapHandler {
                onTapped: {
                    row.forceActiveFocus()
                    root.openGame(row.gameId, row.gameDatabaseId,
                                  row.gameTitle, row.gamePlatform,
                                  row.gameLocal, row.gameDownloadable)
                }
            }
            Keys.onPressed: event => {
                if (event.key === Qt.Key_Return || event.key === Qt.Key_Enter
                        || event.key === Qt.Key_Space) {
                    root.openGame(row.gameId, row.gameDatabaseId,
                                  row.gameTitle, row.gamePlatform,
                                  row.gameLocal, row.gameDownloadable)
                    event.accepted = true
                } else if (event.key === Qt.Key_F && !row.favoriteBusy) {
                    library.set_favorite(row.gameId, !row.favorite)
                    event.accepted = true
                }
            }
            Rectangle {
                id: listArtwork
                width: 38
                height: 38
                radius: 8
                anchors.left: parent.left
                anchors.leftMargin: 12
                anchors.verticalCenter: parent.verticalCenter
                color: root.accentFor(row.gameTitle)
                clip: true
                Image {
                    id: listImage
                    anchors.fill: parent
                    source: row.artworkUrl
                    asynchronous: true
                    cache: true
                    mipmap: true
                    autoTransform: true
                    fillMode: Image.PreserveAspectCrop
                    sourceSize.width: 76
                    sourceSize.height: 76
                    opacity: status === Image.Ready ? 1 : 0
                }
                Text {
                    anchors.centerIn: parent
                    text: row.gameTitle.length > 0 ? row.gameTitle.charAt(0).toUpperCase() : "?"
                    color: "white"
                    font.weight: Font.Black
                    font.pixelSize: 18
                    visible: listImage.status !== Image.Ready
                }
            }
            Column {
                anchors.left: parent.left
                anchors.leftMargin: 62
                anchors.right: listFavorite.left
                anchors.rightMargin: 20
                anchors.verticalCenter: parent.verticalCenter
                spacing: 3
                Text {
                    width: parent.width
                    text: row.gameTitle
                    color: root.ink
                    font.pixelSize: 14
                    font.weight: Font.DemiBold
                    elide: Text.ElideRight
                }
                Text {
                    width: parent.width
                    text: row.gamePlatform.length > 0 ? row.gamePlatform : "Unassigned platform"
                    color: root.muted
                    font.pixelSize: 11
                    elide: Text.ElideRight
                }
            }
            RoundButton {
                id: listFavorite
                anchors.right: stateLabel.left
                anchors.rightMargin: 10
                anchors.verticalCenter: parent.verticalCenter
                width: 32
                height: 32
                enabled: !row.favoriteBusy
                text: row.favoriteBusy ? "…" : row.favorite ? "★" : "☆"
                flat: true
                font.pixelSize: 16
                onClicked: library.set_favorite(row.gameId, !row.favorite)
                ToolTip.visible: hovered
                ToolTip.text: row.favorite ? "Remove from Favorites" : "Add to Favorites"
            }
            Text {
                id: stateLabel
                anchors.right: parent.right
                anchors.rightMargin: 18
                anchors.verticalCenter: parent.verticalCenter
                text: row.gameLocal ? "INSTALLED" : row.gameDownloadable ? "AVAILABLE" : row.gameStatus.toUpperCase()
                color: row.gameLocal ? root.accentCool : row.gameDownloadable ? root.accent : root.muted
                font.pixelSize: 10
                font.weight: Font.Bold
                font.letterSpacing: 0.8
            }
        }
    }

    Rectangle {
        id: header
        anchors.left: parent.left
        anchors.right: parent.right
        anchors.top: parent.top
        height: 70
        color: "#101620"
        border.color: root.line

        Row {
            anchors.left: parent.left
            anchors.leftMargin: 22
            anchors.verticalCenter: parent.verticalCenter
            spacing: 11
            Rectangle {
                width: 35
                height: 35
                radius: 10
                color: root.accent
                Text {
                    anchors.centerIn: parent
                    text: "L"
                    color: "#18120b"
                    font.pixelSize: 20
                    font.weight: Font.Black
                }
            }
            Column {
                anchors.verticalCenter: parent.verticalCenter
                spacing: -2
                Text {
                    text: "LUNCHBOX"
                    color: root.ink
                    font.pixelSize: 17
                    font.weight: Font.Black
                    font.letterSpacing: 1.2
                }
                Text {
                    text: "PLAY WITHOUT FRICTION"
                    color: root.muted
                    font.pixelSize: 8
                    font.weight: Font.Bold
                    font.letterSpacing: 1.1
                }
            }
        }

        TextField {
            id: searchField
            anchors.centerIn: parent
            width: Math.min(460, root.width * 0.34)
            height: 40
            leftPadding: 42
            rightPadding: 14
            placeholderText: "Search games and platforms"
            placeholderTextColor: "#687488"
            color: root.ink
            selectionColor: root.accent
            selectedTextColor: "#15100a"
            font.pixelSize: 13
            onTextEdited: root.scheduleFilter()
            background: Rectangle {
                radius: 11
                color: "#0a0f16"
                border.color: searchField.activeFocus ? root.accent : root.line
                border.width: searchField.activeFocus ? 2 : 1
                Text {
                    anchors.left: parent.left
                    anchors.leftMargin: 15
                    anchors.verticalCenter: parent.verticalCenter
                    text: "⌕"
                    color: root.muted
                    font.pixelSize: 20
                }
            }
        }

        Row {
            anchors.right: parent.right
            anchors.rightMargin: 20
            anchors.verticalCenter: parent.verticalCenter
            spacing: 9
            HeaderButton {
                text: "Minerva  " + library.downloadable_game_count
                visible: root.width >= 1280
                active: root.availability === "downloadable"
                onClicked: root.selectLibrary(active ? "" : "downloadable")
            }
            HeaderButton {
                text: "↓  " + downloadQueue.active_count
                active: downloadsDrawer.opened
                onClicked: downloadsDrawer.open()
            }
            HeaderButton {
                text: "⚙"
                implicitWidth: 42
                leftPadding: 0
                rightPadding: 0
                onClicked: settingsDialog.open()
                ToolTip.visible: hovered
                ToolTip.text: "Settings"
            }
            HeaderButton {
                text: library.loading ? "…" : "↻"
                implicitWidth: 42
                leftPadding: 0
                rightPadding: 0
                enabled: !library.loading
                onClicked: library.reload()
                ToolTip.visible: hovered
                ToolTip.text: "Refresh library"
            }
        }
    }

    Rectangle {
        id: sidebar
        anchors.left: parent.left
        anchors.top: header.bottom
        anchors.bottom: statusBar.top
        width: 254
        color: root.panel
        border.color: root.line

        Column {
            id: libraryNav
            anchors.left: parent.left
            anchors.right: parent.right
            anchors.top: parent.top
            anchors.margins: 13
            spacing: 3

            Text {
                text: "LIBRARY"
                color: "#687488"
                font.pixelSize: 10
                font.weight: Font.Bold
                font.letterSpacing: 1.4
                leftPadding: 13
                topPadding: 8
                bottomPadding: 7
            }
            NavButton {
                label: "All Games"
                glyph: "▦"
                count: library.game_count.toString()
                active: root.selectedPlatform === "" && root.availability === ""
                onClicked: root.selectLibrary("")
            }
            NavButton {
                label: "My Collection"
                glyph: "◆"
                count: library.local_game_count.toString()
                active: root.selectedPlatform === "" && root.availability === "local"
                onClicked: root.selectLibrary("local")
            }
            NavButton {
                label: "Favorites"
                glyph: "★"
                count: library.favorite_count.toString()
                active: root.selectedPlatform === "" && root.availability === "favorites"
                onClicked: root.selectLibrary("favorites")
            }
            NavButton {
                label: "Recently Played"
                glyph: "◷"
                count: library.recent_count.toString()
                active: root.selectedPlatform === "" && root.availability === "recent"
                onClicked: root.selectLibrary("recent")
            }
            NavButton {
                label: "Minerva"
                glyph: "↓"
                count: library.downloadable_game_count.toString()
                active: root.availability === "downloadable"
                onClicked: root.selectLibrary("downloadable")
            }
            NavButton {
                label: "Import ROMs"
                glyph: "+"
                onClicked: root.openImportDialog()
            }
        }

        Item {
            id: collectionsHeader
            anchors.left: parent.left
            anchors.right: parent.right
            anchors.top: libraryNav.bottom
            anchors.topMargin: 15
            height: 29

            Text {
                anchors.left: parent.left
                anchors.leftMargin: 26
                anchors.verticalCenter: parent.verticalCenter
                text: "COLLECTIONS"
                color: "#687488"
                font.pixelSize: 10
                font.weight: Font.Bold
                font.letterSpacing: 1.4
            }
            Row {
                anchors.right: parent.right
                anchors.rightMargin: 16
                anchors.verticalCenter: parent.verticalCenter
                spacing: 2
                ToolButton {
                    text: "+"
                    width: 28
                    height: 28
                    enabled: !library.collection_busy
                    onClicked: root.openNewCollectionDialog()
                    ToolTip.visible: hovered
                    ToolTip.text: "New collection"
                }
                ToolButton {
                    text: "⋯"
                    width: 28
                    height: 28
                    enabled: library.collection_count > 0
                    onClicked: manageCollectionsDialog.open()
                    ToolTip.visible: hovered
                    ToolTip.text: "Manage collections"
                }
            }
        }

        ListView {
            id: collectionList
            anchors.left: parent.left
            anchors.right: parent.right
            anchors.top: collectionsHeader.bottom
            anchors.leftMargin: 13
            anchors.rightMargin: 13
            height: Math.min(contentHeight, 142)
            clip: true
            reuseItems: true
            spacing: 3
            model: library.collection_count
            ScrollBar.vertical: ScrollBar { policy: ScrollBar.AsNeeded }
            delegate: NavButton {
                required property int index
                property int revision: library.collection_revision
                label: {
                    revision
                    return library.collection_name_at(index)
                }
                glyph: "▣"
                count: library.collection_game_count_at(index).toString()
                active: root.selectedCollectionId === library.collection_id_at(index)
                onClicked: root.selectCollection(library.collection_id_at(index), label)
            }
        }

        Text {
            id: platformsLabel
            anchors.left: parent.left
            anchors.leftMargin: 26
            anchors.top: collectionList.bottom
            anchors.topMargin: 17
            text: "PLATFORMS"
            color: "#687488"
            font.pixelSize: 10
            font.weight: Font.Bold
            font.letterSpacing: 1.4
        }

        ListView {
            id: platformList
            anchors.left: parent.left
            anchors.right: parent.right
            anchors.top: platformsLabel.bottom
            anchors.bottom: parent.bottom
            anchors.margins: 13
            anchors.topMargin: 10
            clip: true
            reuseItems: true
            spacing: 3
            model: library.platform_count
            ScrollBar.vertical: ScrollBar { policy: ScrollBar.AsNeeded }
            delegate: NavButton {
                required property int index
                property int revision: library.platform_revision
                label: {
                    revision
                    return library.platform_name_at(index)
                }
                glyph: "·"
                count: library.platform_game_count_at(index).toString()
                active: root.selectedPlatform === label
                onClicked: {
                    root.selectedPlatform = label
                    root.scheduleFilter()
                }
            }
        }
    }

    Item {
        id: content
        anchors.left: sidebar.right
        anchors.right: detailsPane.left
        anchors.top: header.bottom
        anchors.bottom: statusBar.top

        ColumnLayout {
            anchors.fill: parent
            anchors.leftMargin: 27
            anchors.rightMargin: 27
            anchors.topMargin: 23
            anchors.bottomMargin: 16
            spacing: 15

            RowLayout {
                Layout.fillWidth: true
                spacing: 10
                Column {
                    Layout.fillWidth: true
                    Layout.minimumWidth: 0
                    spacing: 3
                    Text {
                        width: parent.width
                        text: root.platformHeading()
                        color: root.ink
                        font.pixelSize: 27
                        font.weight: Font.Bold
                        elide: Text.ElideRight
                    }
                    Text {
                        text: library.filtering ? "Updating results…" : library.filtered_count + " games"
                        color: root.muted
                        font.pixelSize: 12
                    }
                }
                StatusPill {
                    visible: content.width > 900
                    label: "local files"
                    value: library.local_file_count.toString()
                }
                StatusPill {
                    visible: content.width > 900
                    label: "offers"
                    value: library.offer_count.toString()
                }
                StatusPill {
                    visible: content.width > 900
                    label: library.media_loading ? "artwork" : "with art"
                    value: library.media_loading ? "…" : library.media_game_count.toString()
                }
                HeaderButton {
                    text: root.activeFilterCount > 0
                          ? "Filters  " + root.activeFilterCount : "Filters"
                    active: root.activeFilterCount > 0 || filterPopup.opened
                    onClicked: filterPopup.opened ? filterPopup.close() : filterPopup.open()
                }
                HeaderButton {
                    text: "View"
                    active: viewPopup.opened
                    onClicked: viewPopup.opened ? viewPopup.close() : viewPopup.open()
                    ToolTip.visible: hovered
                    ToolTip.text: root.artworkLabel(library.artwork_type)
                                  + " · " + library.grid_zoom + "%"
                }
                Row {
                    spacing: 4
                    HeaderButton {
                        text: "▦"
                        active: root.gridMode
                        implicitWidth: 41
                        leftPadding: 0
                        rightPadding: 0
                        onClicked: root.gridMode = true
                    }
                    HeaderButton {
                        text: "☷"
                        active: !root.gridMode
                        implicitWidth: 41
                        leftPadding: 0
                        rightPadding: 0
                        onClicked: root.gridMode = false
                    }
                }
            }

            Rectangle {
                Layout.fillWidth: true
                Layout.preferredHeight: 1
                color: root.line
            }

            Loader {
                id: gameViewLoader
                Layout.fillWidth: true
                Layout.fillHeight: true
                active: library.ready && library.filtered_count > 0
                sourceComponent: root.gridMode ? gameGridComponent : gameListComponent
            }

            Item {
                Layout.fillWidth: true
                Layout.fillHeight: true
                visible: !gameViewLoader.active

                Column {
                    anchors.centerIn: parent
                    width: Math.min(580, parent.width - 60)
                    spacing: 13
                    Rectangle {
                        anchors.horizontalCenter: parent.horizontalCenter
                        width: 62
                        height: 62
                        radius: 19
                        color: "#1a2230"
                        border.color: root.line
                        Text {
                            anchors.centerIn: parent
                            text: library.loading ? "↻" : library.ready ? "◇" : "!"
                            color: library.loading ? root.accent : root.accentCool
                            font.pixelSize: 29
                            font.weight: Font.Bold
                        }
                    }
                    Text {
                        anchors.horizontalCenter: parent.horizontalCenter
                        text: library.loading ? "Opening your library" :
                              !library.ready ? "Catalog unavailable" :
                              library.game_count === 0 ? "Your library is ready for games" :
                              "No games match these filters"
                        color: root.ink
                        font.pixelSize: 21
                        font.weight: Font.Bold
                    }
                    Text {
                        width: parent.width
                        horizontalAlignment: Text.AlignHCenter
                        wrapMode: Text.WordWrap
                        text: library.loading ? "The window stays responsive while SQLite is read on a worker thread." :
                              !library.ready ? library.status_message :
                              library.game_count === 0 ?
                                  "Add a discovery database or import local games. Minerva availability is layered over the catalog without treating provider filenames as canonical identities." :
                                  "Try a different platform, availability option, or search."
                        color: root.muted
                        font.pixelSize: 13
                        lineHeight: 1.35
                    }
                    Text {
                        visible: !library.ready
                        width: parent.width
                        horizontalAlignment: Text.AlignHCenter
                        text: library.database_path.length > 0 ? library.database_path :
                              "Use --database PATH or LUNCHBOX_DATABASE"
                        color: "#657186"
                        font.pixelSize: 11
                        elide: Text.ElideMiddle
                    }
                }
            }
        }

        Component { id: gameGridComponent; GameGrid {} }
        Component { id: gameListComponent; GameList {} }
    }

    Rectangle {
        id: detailsPane
        anchors.right: parent.right
        anchors.top: header.bottom
        anchors.bottom: statusBar.top
        width: gameDetails.panel_open ? Math.min(440, Math.max(350, root.width * 0.32)) : 0
        visible: width > 0
        clip: true
        color: "#101620"
        border.color: root.line

        Behavior on width {
            NumberAnimation { duration: 150; easing.type: Easing.OutCubic }
        }

        Rectangle {
            id: detailHeader
            anchors.left: parent.left
            anchors.right: parent.right
            anchors.top: parent.top
            height: 56
            color: root.panel
            border.color: root.line

            Text {
                anchors.left: parent.left
                anchors.leftMargin: 20
                anchors.verticalCenter: parent.verticalCenter
                text: "GAME DETAILS"
                color: root.muted
                font.pixelSize: 10
                font.weight: Font.Bold
                font.letterSpacing: 1.4
            }
            RoundButton {
                id: detailFavoriteButton
                anchors.right: detailCollectionsButton.left
                anchors.rightMargin: 4
                anchors.verticalCenter: parent.verticalCenter
                width: 34
                height: 34
                text: root.selectedFavoritePending ? "…" : root.selectedFavorite ? "★" : "☆"
                flat: true
                enabled: root.selectedGameId.length > 0 && !root.selectedFavoritePending
                font.pixelSize: 17
                onClicked: library.set_favorite(root.selectedGameId,
                                                !root.selectedFavorite)
                ToolTip.visible: hovered
                ToolTip.text: root.selectedFavorite
                              ? "Remove from Favorites" : "Add to Favorites"
            }
            RoundButton {
                id: detailCollectionsButton
                anchors.right: closeDetailsButton.left
                anchors.rightMargin: 4
                anchors.verticalCenter: parent.verticalCenter
                width: 34
                height: 34
                text: "▣"
                flat: true
                enabled: root.selectedGameId.length > 0
                         && !library.collection_busy
                font.pixelSize: 15
                onClicked: {
                    if (library.collection_count > 0)
                        collectionMembershipPopup.open()
                    else
                        root.openNewCollectionDialog()
                }
                ToolTip.visible: hovered
                ToolTip.text: library.collection_count > 0
                              ? "Add to collections"
                              : "Create a collection first"
            }
            RoundButton {
                id: closeDetailsButton
                anchors.right: parent.right
                anchors.rightMargin: 12
                anchors.verticalCenter: parent.verticalCenter
                width: 34
                height: 34
                text: "×"
                flat: true
                font.pixelSize: 20
                onClicked: {
                    root.selectedGameId = ""
                    root.selectedDatabaseId = 0
                    root.selectedArtworkUrl = ""
                    root.selectedFanartUrl = ""
                    root.selectedArtworkSource = ""
                    root.selectedHeroArtworkIndex = 0
                    root.selectedHeroArtworkCount = 0
                    gameDetails.close_panel()
                }
                ToolTip.visible: hovered
                ToolTip.text: "Close details"
            }
        }

        Popup {
            id: collectionMembershipPopup
            x: Math.max(12, detailsPane.width - width - 14)
            y: detailHeader.height - 4
            width: Math.min(320, detailsPane.width - 24)
            height: Math.min(390, membershipColumn.implicitHeight + 28)
            padding: 14
            modal: false
            closePolicy: Popup.CloseOnEscape | Popup.CloseOnReleaseOutside
            background: Rectangle {
                color: root.panelRaised
                radius: 12
                border.color: root.line
            }
            contentItem: Column {
                id: membershipColumn
                spacing: 8
                Text {
                    width: parent.width
                    text: "ADD TO COLLECTION"
                    color: root.accent
                    font.pixelSize: 10
                    font.weight: Font.Bold
                    font.letterSpacing: 1.2
                }
                ListView {
                    width: parent.width
                    height: Math.min(contentHeight, 278)
                    clip: true
                    spacing: 3
                    model: library.collection_count
                    ScrollBar.vertical: ScrollBar { policy: ScrollBar.AsNeeded }
                    delegate: CheckDelegate {
                        id: membershipDelegate
                        required property int index
                        property int revision: library.collection_revision
                        width: ListView.view.width
                        text: {
                            revision
                            return library.collection_name_at(index)
                        }
                        checked: {
                            revision
                            return library.collection_contains(
                                        library.collection_id_at(index),
                                        root.selectedGameId)
                        }
                        enabled: !library.collection_busy
                        onClicked: {
                            const collectionId = library.collection_id_at(index)
                            library.set_collection_membership(
                                        collectionId, root.selectedGameId,
                                        !library.collection_contains(
                                            collectionId, root.selectedGameId))
                        }
                    }
                }
                Button {
                    width: parent.width
                    text: "+  New collection"
                    enabled: !library.collection_busy
                    onClicked: {
                        collectionMembershipPopup.close()
                        root.openNewCollectionDialog()
                    }
                }
            }
        }

        ScrollView {
            id: detailScroll
            anchors.left: parent.left
            anchors.right: parent.right
            anchors.top: detailHeader.bottom
            anchors.bottom: parent.bottom
            clip: true
            ScrollBar.horizontal.policy: ScrollBar.AlwaysOff
            ScrollBar.vertical.policy: ScrollBar.AsNeeded

            Column {
                width: detailScroll.availableWidth
                spacing: 0

                Rectangle {
                    id: detailArtwork
                    width: parent.width
                    height: 214
                    color: root.accentFor(gameDetails.title)
                    clip: true
                    gradient: Gradient {
                        GradientStop { position: 0; color: Qt.lighter(detailArtwork.color, 1.15) }
                        GradientStop { position: 1; color: Qt.darker(detailArtwork.color, 1.8) }
                    }
                    Image {
                        id: detailImage
                        anchors.fill: parent
                        source: root.selectedFanartUrl.toString().length > 0
                                ? root.selectedFanartUrl : root.selectedArtworkUrl
                        asynchronous: true
                        cache: true
                        mipmap: true
                        autoTransform: true
                        fillMode: root.selectedFanartUrl.toString().length > 0
                                  ? Image.PreserveAspectCrop : Image.PreserveAspectFit
                        sourceSize.width: Math.max(1, Math.round(width * 2))
                        sourceSize.height: Math.max(1, Math.round(height * 2))
                        opacity: status === Image.Ready ? 1 : 0
                        visible: !root.selectedBox3d
                        Behavior on opacity { NumberAnimation { duration: 150 } }
                    }
                    Box3DViewer {
                        id: detailBox3d
                        anchors.fill: parent
                        frontSource: root.selectedBoxFrontUrl
                        backSource: root.selectedBoxBackUrl.toString().length > 0
                                    ? root.selectedBoxBackUrl
                                    : root.selectedBoxFrontUrl
                        visible: root.selectedBox3d
                    }
                    Text {
                        anchors.centerIn: parent
                        text: gameDetails.title.length > 0 ? gameDetails.title.charAt(0).toUpperCase() : "?"
                        color: "#35ffffff"
                        font.pixelSize: 118
                        font.weight: Font.Black
                        visible: !root.selectedBox3d
                                 && detailImage.status !== Image.Ready
                    }
                    Rectangle {
                        visible: !root.selectedBox3d
                        anchors.left: parent.left
                        anchors.bottom: parent.bottom
                        width: parent.width
                        height: 112
                        gradient: Gradient {
                            GradientStop { position: 0; color: "transparent" }
                            GradientStop { position: 1; color: "#d9101620" }
                        }
                    }
                    Rectangle {
                        visible: !root.selectedBox3d
                                 && detailImage.status === Image.Ready
                                 && root.selectedArtworkSource.length > 0
                        anchors.left: parent.left
                        anchors.top: parent.top
                        anchors.margins: 12
                        width: detailSource.implicitWidth + 16
                        height: 24
                        radius: 7
                        color: "#c9101620"
                        border.color: "#5b687d"
                        Text {
                            id: detailSource
                            anchors.centerIn: parent
                            text: root.selectedArtworkSource.toUpperCase()
                            color: "#d7deea"
                            font.pixelSize: 8
                            font.weight: Font.Bold
                            font.letterSpacing: 0.7
                        }
                    }
                    BusyIndicator {
                        anchors.centerIn: parent
                        running: gameDetails.loading
                        visible: running && !root.selectedBox3d
                    }
                    Button {
                        anchors.right: parent.right
                        anchors.top: parent.top
                        anchors.margins: 11
                        visible: root.selectedBoxFrontUrl.toString().length > 0
                        text: root.selectedBox3d ? "ARTWORK" : "3D BOX"
                        font.pixelSize: 9
                        font.weight: Font.Bold
                        onClicked: root.selectedBox3d = !root.selectedBox3d
                        background: Rectangle {
                            radius: 8
                            color: root.selectedBox3d ? root.accent : "#d5101620"
                            border.color: root.selectedBox3d ? root.accent : "#627087"
                        }
                    }
                    Row {
                        anchors.right: parent.right
                        anchors.bottom: parent.bottom
                        anchors.margins: 9
                        spacing: 5
                        visible: !root.selectedBox3d
                                 && root.selectedDatabaseId > 0
                                 && (root.selectedHeroArtworkCount > 1
                                     || library.media_retrieval_enabled)

                        RoundButton {
                            width: 30
                            height: 30
                            visible: root.selectedHeroArtworkCount > 1
                            text: "‹"
                            font.pixelSize: 20
                            Accessible.name: "Previous artwork"
                            onClicked: root.rotateSelectedHeroArtwork(-1)
                            ToolTip.visible: hovered
                            ToolTip.text: "Previous cached artwork"
                        }
                        Rectangle {
                            width: artworkPosition.implicitWidth + 16
                            height: 30
                            radius: 9
                            visible: root.selectedHeroArtworkCount > 1
                            color: "#d5101620"
                            border.color: "#627087"
                            Text {
                                id: artworkPosition
                                anchors.centerIn: parent
                                text: (root.selectedHeroArtworkIndex + 1)
                                      + " / " + root.selectedHeroArtworkCount
                                color: "#d7deea"
                                font.pixelSize: 9
                                font.weight: Font.Bold
                            }
                        }
                        RoundButton {
                            width: 30
                            height: 30
                            visible: root.selectedHeroArtworkCount > 1
                            text: "›"
                            font.pixelSize: 20
                            Accessible.name: "Next artwork"
                            onClicked: root.rotateSelectedHeroArtwork(1)
                            ToolTip.visible: hovered
                            ToolTip.text: "Next cached artwork"
                        }
                        RoundButton {
                            width: 30
                            height: 30
                            visible: library.media_retrieval_enabled
                            text: "↻"
                            Accessible.name: "Refresh artwork from LibRetro"
                            onClicked: root.refreshHeroArtworkFromProvider()
                            ToolTip.visible: hovered
                            ToolTip.text: "Fetch a fresh LibRetro copy without deleting other artwork"
                        }
                    }
                }

                Column {
                    x: 20
                    width: parent.width - 40
                    spacing: 12

                    Text {
                        width: parent.width
                        text: gameDetails.title
                        color: root.ink
                        font.pixelSize: 24
                        font.weight: Font.Bold
                        wrapMode: Text.WordWrap
                    }
                    Row {
                        width: parent.width
                        spacing: 8
                        Text {
                            text: gameDetails.platform.length > 0 ? gameDetails.platform : "Unassigned platform"
                            color: root.muted
                            font.pixelSize: 12
                        }
                        Rectangle {
                            width: stateText.implicitWidth + 14
                            height: 24
                            radius: 7
                            color: gameDetails.local ? "#1d3d35" : gameDetails.downloadable ? "#303540" : "#242b36"
                            border.color: gameDetails.local ? root.accentCool : gameDetails.downloadable ? root.accent : root.line
                            Text {
                                id: stateText
                                anchors.centerIn: parent
                                text: gameDetails.local ? "INSTALLED" : gameDetails.downloadable ? "MINERVA" : "CATALOG"
                                color: gameDetails.local ? root.accentCool : gameDetails.downloadable ? root.accent : root.muted
                                font.pixelSize: 9
                                font.weight: Font.Bold
                                font.letterSpacing: 0.7
                            }
                        }
                    }

                    Rectangle {
                        width: parent.width
                        height: statusMessage.implicitHeight + 22
                        radius: 9
                        color: "#151d29"
                        border.color: root.line
                        Row {
                            anchors.left: parent.left
                            anchors.right: parent.right
                            anchors.verticalCenter: parent.verticalCenter
                            anchors.leftMargin: 11
                            anchors.rightMargin: 11
                            spacing: 9
                            BusyIndicator {
                                id: statusBusy
                                width: 18
                                height: 18
                                running: gameDetails.loading || gameDetails.torrent_loading
                                         || gameDetails.prepare_busy
                                         || gameDetails.launch_discovery_busy
                                         || gameDetails.launch_busy
                                visible: running
                            }
                            Text {
                                id: statusMessage
                                width: parent.width - (statusBusy.visible ? statusBusy.width + 9 : 0)
                                text: gameDetails.message
                                color: root.muted
                                font.pixelSize: 11
                                wrapMode: Text.WordWrap
                            }
                        }
                    }

                    Rectangle {
                        visible: !gameDetails.loading && gameDetails.activity_visible
                        width: parent.width
                        height: activityColumn.implicitHeight + 24
                        radius: 11
                        color: "#171f2b"
                        border.color: "#354155"

                        Column {
                            id: activityColumn
                            anchors.left: parent.left
                            anchors.right: parent.right
                            anchors.top: parent.top
                            anchors.margins: 12
                            spacing: 10

                            Row {
                                width: parent.width
                                spacing: 8
                                Text {
                                    width: parent.width - activityBusy.width - 8
                                    text: "MY ACTIVITY"
                                    color: root.accent
                                    font.pixelSize: 10
                                    font.weight: Font.Bold
                                    font.letterSpacing: 1.1
                                    verticalAlignment: Text.AlignVCenter
                                }
                                BusyIndicator {
                                    id: activityBusy
                                    width: 18
                                    height: 18
                                    running: gameDetails.activity_busy
                                    visible: running
                                }
                            }

                            Row {
                                width: parent.width
                                spacing: 8
                                Column {
                                    width: (parent.width - 16) / 3
                                    spacing: 2
                                    Text {
                                        text: gameDetails.play_count.toString()
                                        color: root.ink
                                        font.pixelSize: 17
                                        font.weight: Font.Bold
                                        font.features: { "tnum": 1 }
                                    }
                                    Text {
                                        text: gameDetails.play_count === 1 ? "PLAY" : "PLAYS"
                                        color: root.muted
                                        font.pixelSize: 8
                                        font.weight: Font.Bold
                                        font.letterSpacing: 0.6
                                    }
                                }
                                Column {
                                    width: (parent.width - 16) / 3
                                    spacing: 2
                                    Text {
                                        width: parent.width
                                        text: gameDetails.play_time
                                        color: root.ink
                                        font.pixelSize: 13
                                        font.weight: Font.Bold
                                        elide: Text.ElideRight
                                    }
                                    Text {
                                        text: "PLAY TIME"
                                        color: root.muted
                                        font.pixelSize: 8
                                        font.weight: Font.Bold
                                        font.letterSpacing: 0.6
                                    }
                                }
                                Column {
                                    width: (parent.width - 16) / 3
                                    spacing: 2
                                    Text {
                                        width: parent.width
                                        text: gameDetails.last_played
                                        color: root.ink
                                        font.pixelSize: 13
                                        font.weight: Font.Bold
                                        elide: Text.ElideRight
                                    }
                                    Text {
                                        text: "LAST PLAYED"
                                        color: root.muted
                                        font.pixelSize: 8
                                        font.weight: Font.Bold
                                        font.letterSpacing: 0.6
                                    }
                                }
                            }

                            Row {
                                width: parent.width
                                spacing: 10
                                Text {
                                    width: 88
                                    height: completionPicker.height
                                    text: "COMPLETION"
                                    color: root.muted
                                    font.pixelSize: 8
                                    font.weight: Font.Bold
                                    font.letterSpacing: 0.6
                                    verticalAlignment: Text.AlignVCenter
                                }
                                ComboBox {
                                    id: completionPicker
                                    width: parent.width - 98
                                    height: 34
                                    enabled: !gameDetails.activity_busy
                                    model: root.completionChoices
                                    textRole: "label"
                                    currentIndex: root.completionIndex(gameDetails.completion_state)
                                    onActivated: function(index) {
                                        gameDetails.save_completion_state(
                                                    root.completionChoices[index].key)
                                    }
                                    contentItem: Text {
                                        leftPadding: 10
                                        rightPadding: 28
                                        text: completionPicker.displayText
                                        color: completionPicker.enabled ? root.ink : root.muted
                                        font.pixelSize: 10
                                        font.weight: Font.Medium
                                        verticalAlignment: Text.AlignVCenter
                                        elide: Text.ElideRight
                                    }
                                    background: Rectangle {
                                        radius: 7
                                        color: "#101721"
                                        border.color: completionPicker.activeFocus
                                                      ? root.accent : root.line
                                    }
                                }
                            }
                        }
                    }

                    Rectangle {
                        id: mediaCard
                        visible: !gameDetails.loading && gameDetails.media_visible
                        width: parent.width
                        height: mediaColumn.implicitHeight + 24
                        radius: 11
                        color: "#111a26"
                        border.color: "#354155"
                        clip: true

                        Column {
                            id: mediaColumn
                            anchors.left: parent.left
                            anchors.right: parent.right
                            anchors.top: parent.top
                            anchors.margins: 12
                            spacing: 10

                            Row {
                                width: parent.width
                                spacing: 8
                                Text {
                                    width: parent.width - mediaSourceBadge.width - 8
                                    text: "GAME MEDIA"
                                    color: root.accent
                                    font.pixelSize: 10
                                    font.weight: Font.Bold
                                    font.letterSpacing: 1.1
                                }
                                Rectangle {
                                    id: mediaSourceBadge
                                    visible: gameDetails.video_available
                                    width: visible ? mediaSourceText.implicitWidth + 14 : 0
                                    height: 20
                                    radius: 6
                                    color: "#202b3a"
                                    border.color: root.line
                                    Text {
                                        id: mediaSourceText
                                        anchors.centerIn: parent
                                        text: gameDetails.video_source.toUpperCase()
                                        color: root.muted
                                        font.pixelSize: 8
                                        font.weight: Font.Bold
                                        font.letterSpacing: 0.6
                                    }
                                }
                            }

                            Rectangle {
                                id: detailVideoFrame
                                visible: gameDetails.video_available
                                width: parent.width
                                height: visible ? Math.round(width * 0.57) : 0
                                radius: 9
                                color: "#070a0f"
                                border.color: "#303b4d"
                                clip: true

                                VideoOutput {
                                    id: detailVideoOutput
                                    anchors.fill: parent
                                    fillMode: VideoOutput.PreserveAspectFit
                                }

                                BusyIndicator {
                                    anchors.centerIn: parent
                                    width: 32
                                    height: 32
                                    running: gameVideoPlayer.mediaStatus === MediaPlayer.LoadingMedia
                                             || gameVideoPlayer.mediaStatus === MediaPlayer.BufferingMedia
                                    visible: running
                                }

                                Rectangle {
                                    anchors.left: parent.left
                                    anchors.right: parent.right
                                    anchors.bottom: parent.bottom
                                    height: 54
                                    gradient: Gradient {
                                        GradientStop { position: 0; color: "transparent" }
                                        GradientStop { position: 1; color: "#e5070a0f" }
                                    }
                                }

                                MouseArea {
                                    anchors.fill: parent
                                    anchors.bottomMargin: videoControls.height
                                    cursorShape: Qt.PointingHandCursor
                                    onClicked: {
                                        if (gameVideoPlayer.playbackState
                                                === MediaPlayer.PlayingState)
                                            gameVideoPlayer.pause()
                                        else
                                            gameVideoPlayer.play()
                                    }
                                }

                                RowLayout {
                                    id: videoControls
                                    anchors.left: parent.left
                                    anchors.right: parent.right
                                    anchors.bottom: parent.bottom
                                    anchors.leftMargin: 8
                                    anchors.rightMargin: 8
                                    height: 42
                                    spacing: 6

                                    Button {
                                        Layout.preferredWidth: 38
                                        Layout.preferredHeight: 30
                                        text: gameVideoPlayer.playbackState
                                              === MediaPlayer.PlayingState ? "Ⅱ" : "▶"
                                        onClicked: {
                                            if (gameVideoPlayer.playbackState
                                                    === MediaPlayer.PlayingState)
                                                gameVideoPlayer.pause()
                                            else
                                                gameVideoPlayer.play()
                                        }
                                        background: Rectangle {
                                            radius: 7
                                            color: parent.down ? "#3a4556" : "#283343"
                                        }
                                        contentItem: Text {
                                            text: parent.text
                                            color: root.ink
                                            font.pixelSize: 12
                                            font.weight: Font.Bold
                                            horizontalAlignment: Text.AlignHCenter
                                            verticalAlignment: Text.AlignVCenter
                                        }
                                    }

                                    Slider {
                                        id: videoPosition
                                        Layout.fillWidth: true
                                        from: 0
                                        to: Math.max(1, gameVideoPlayer.duration)
                                        onMoved: {
                                            if (gameVideoPlayer.seekable)
                                                gameVideoPlayer.position = value
                                        }
                                        Binding on value {
                                            when: !videoPosition.pressed
                                            value: gameVideoPlayer.position
                                        }
                                        background: Rectangle {
                                            x: videoPosition.leftPadding
                                            y: videoPosition.topPadding
                                               + videoPosition.availableHeight / 2 - height / 2
                                            width: videoPosition.availableWidth
                                            height: 4
                                            radius: 2
                                            color: "#465166"
                                            Rectangle {
                                                width: videoPosition.visualPosition * parent.width
                                                height: parent.height
                                                radius: 2
                                                color: root.accent
                                            }
                                        }
                                        handle: Rectangle {
                                            x: videoPosition.leftPadding
                                               + videoPosition.visualPosition
                                                 * (videoPosition.availableWidth - width)
                                            y: videoPosition.topPadding
                                               + videoPosition.availableHeight / 2 - height / 2
                                            width: 12
                                            height: 12
                                            radius: 6
                                            color: root.ink
                                        }
                                    }

                                    Text {
                                        Layout.preferredWidth: 68
                                        text: root.formatMediaTime(gameVideoPlayer.position)
                                              + " / "
                                              + root.formatMediaTime(gameVideoPlayer.duration)
                                        color: "#d4dae4"
                                        font.pixelSize: 8
                                        font.features: { "tnum": 1 }
                                        horizontalAlignment: Text.AlignHCenter
                                    }

                                    Button {
                                        Layout.preferredWidth: 34
                                        Layout.preferredHeight: 30
                                        text: gameVideoAudio.muted ? "MUTE" : "SOUND"
                                        onClicked: gameVideoAudio.muted = !gameVideoAudio.muted
                                        ToolTip.visible: hovered
                                        ToolTip.text: gameVideoAudio.muted ? "Turn sound on" : "Mute video"
                                        background: Rectangle {
                                            radius: 7
                                            color: parent.down ? "#3a4556" : "#283343"
                                        }
                                        contentItem: Text {
                                            text: parent.text
                                            color: gameVideoAudio.muted ? root.muted : root.accentCool
                                            font.pixelSize: 6
                                            font.weight: Font.Bold
                                            horizontalAlignment: Text.AlignHCenter
                                            verticalAlignment: Text.AlignVCenter
                                        }
                                    }

                                    Button {
                                        Layout.preferredWidth: 34
                                        Layout.preferredHeight: 30
                                        text: "FULL"
                                        onClicked: mediaFullscreen.open()
                                        ToolTip.visible: hovered
                                        ToolTip.text: "View fullscreen"
                                        background: Rectangle {
                                            radius: 7
                                            color: parent.down ? "#3a4556" : "#283343"
                                        }
                                        contentItem: Text {
                                            text: parent.text
                                            color: root.ink
                                            font.pixelSize: 6
                                            font.weight: Font.Bold
                                            horizontalAlignment: Text.AlignHCenter
                                            verticalAlignment: Text.AlignVCenter
                                        }
                                    }
                                }
                            }

                            RowLayout {
                                visible: gameDetails.video_available
                                         && gameDetails.video_resume_position >= 5000
                                width: parent.width
                                spacing: 8
                                Text {
                                    Layout.fillWidth: true
                                    text: "RESUMED AT "
                                          + root.formatMediaTime(gameDetails.video_resume_position)
                                    color: root.muted
                                    font.pixelSize: 8
                                    font.weight: Font.Bold
                                    font.letterSpacing: 0.6
                                }
                                Button {
                                    text: "START OVER"
                                    enabled: !gameDetails.video_progress_busy
                                    onClicked: {
                                        gameVideoPlayer.position = 0
                                        gameDetails.reset_video_progress(gameVideoPlayer.duration)
                                    }
                                    background: Rectangle {
                                        radius: 7
                                        color: parent.down ? "#303b4b" : "#202a38"
                                        border.color: root.line
                                    }
                                    contentItem: Text {
                                        text: parent.text
                                        color: root.ink
                                        font.pixelSize: 8
                                        font.weight: Font.Bold
                                        horizontalAlignment: Text.AlignHCenter
                                        verticalAlignment: Text.AlignVCenter
                                    }
                                }
                            }

                            Rectangle {
                                visible: gameDetails.manual_available
                                width: parent.width
                                height: visible ? 48 : 0
                                radius: 8
                                color: "#182230"
                                border.color: root.line
                                RowLayout {
                                    anchors.fill: parent
                                    anchors.leftMargin: 12
                                    anchors.rightMargin: 8
                                    spacing: 8
                                    Column {
                                        Layout.fillWidth: true
                                        spacing: 2
                                        Text {
                                            text: "GAME MANUAL"
                                            color: root.ink
                                            font.pixelSize: 10
                                            font.weight: Font.Bold
                                        }
                                        Text {
                                            text: gameDetails.manual_source.toUpperCase()
                                                  + " · OPENS IN YOUR DEFAULT VIEWER"
                                            color: root.muted
                                            font.pixelSize: 7
                                            font.weight: Font.Bold
                                            font.letterSpacing: 0.4
                                        }
                                    }
                                    Button {
                                        Layout.preferredWidth: 74
                                        Layout.preferredHeight: 32
                                        text: "OPEN"
                                        onClicked: {
                                            if (!Qt.openUrlExternally(gameDetails.manual_url))
                                                root.mediaPlaybackMessage =
                                                        "The operating system could not open this manual."
                                        }
                                        background: Rectangle {
                                            radius: 7
                                            color: parent.down ? "#d89444" : root.accent
                                        }
                                        contentItem: Text {
                                            text: parent.text
                                            color: "#17110a"
                                            font.pixelSize: 8
                                            font.weight: Font.Bold
                                            horizontalAlignment: Text.AlignHCenter
                                            verticalAlignment: Text.AlignVCenter
                                        }
                                    }
                                }
                            }

                            Rectangle {
                                visible: !gameDetails.manual_available
                                width: parent.width
                                height: visible ? manualDownloadColumn.implicitHeight + 20 : 0
                                radius: 8
                                color: "#151f2c"
                                border.color: gameDetails.manual_transfer_active
                                              ? root.accentCool : root.line

                                Column {
                                    id: manualDownloadColumn
                                    anchors.left: parent.left
                                    anchors.right: parent.right
                                    anchors.top: parent.top
                                    anchors.margins: 10
                                    spacing: 8

                                    RowLayout {
                                        width: parent.width
                                        spacing: 10
                                        Column {
                                            Layout.fillWidth: true
                                            spacing: 3
                                            Text {
                                                text: "GAME MANUAL"
                                                color: root.ink
                                                font.pixelSize: 10
                                                font.weight: Font.Bold
                                            }
                                            Text {
                                                text: gameDetails.manual_transfer_active
                                                      ? gameDetails.manual_download_state.toUpperCase()
                                                        + " · MINERVA"
                                                      : "NOT CACHED · MINERVA"
                                                color: gameDetails.manual_transfer_active
                                                       ? root.accentCool : root.muted
                                                font.pixelSize: 7
                                                font.weight: Font.Bold
                                                font.letterSpacing: 0.5
                                            }
                                        }
                                        BusyIndicator {
                                            Layout.preferredWidth: 26
                                            Layout.preferredHeight: 26
                                            running: gameDetails.manual_action_busy
                                            visible: running
                                        }
                                        Button {
                                            Layout.preferredWidth: 74
                                            Layout.preferredHeight: 32
                                            text: gameDetails.manual_transfer_active
                                                  ? "CANCEL" : "FIND"
                                            enabled: !gameDetails.manual_action_busy
                                            onClicked: {
                                                if (gameDetails.manual_transfer_active)
                                                    gameDetails.cancel_manual_download()
                                                else
                                                    gameDetails.download_manual()
                                            }
                                            background: Rectangle {
                                                radius: 7
                                                color: parent.down ? "#344254"
                                                                      : gameDetails.manual_transfer_active
                                                                        ? "#263647" : root.accent
                                                border.color: gameDetails.manual_transfer_active
                                                              ? root.accentCool : "transparent"
                                            }
                                            contentItem: Text {
                                                text: parent.text
                                                color: gameDetails.manual_transfer_active
                                                       ? root.ink : "#17110a"
                                                font.pixelSize: 8
                                                font.weight: Font.Bold
                                                horizontalAlignment: Text.AlignHCenter
                                                verticalAlignment: Text.AlignVCenter
                                            }
                                        }
                                    }

                                    ProgressBar {
                                        id: manualDownloadProgress
                                        visible: gameDetails.manual_transfer_active
                                        width: parent.width
                                        height: visible ? 7 : 0
                                        from: 0
                                        to: 100
                                        value: gameDetails.manual_download_progress
                                        background: Rectangle {
                                            implicitHeight: 5
                                            radius: 3
                                            color: "#303b4d"
                                        }
                                        contentItem: Item {
                                            implicitHeight: 5
                                            Rectangle {
                                                width: manualDownloadProgress.visualPosition
                                                       * parent.width
                                                height: parent.height
                                                radius: 3
                                                color: root.accentCool
                                            }
                                        }
                                    }

                                    RowLayout {
                                        width: parent.width
                                        spacing: 8
                                        Text {
                                            Layout.fillWidth: true
                                            text: gameDetails.manual_download_message
                                            color: gameDetails.manual_download_state === "error"
                                                   ? "#f3a49c" : root.muted
                                            font.pixelSize: 8
                                            wrapMode: Text.WordWrap
                                        }
                                        Text {
                                            visible: gameDetails.manual_download_detail.length > 0
                                            text: gameDetails.manual_download_detail
                                            color: root.ink
                                            font.pixelSize: 8
                                            font.features: { "tnum": 1 }
                                        }
                                    }
                                }
                            }

                            Text {
                                width: parent.width
                                text: root.mediaPlaybackMessage.length > 0
                                      ? root.mediaPlaybackMessage : gameDetails.media_message
                                color: root.mediaPlaybackMessage.length > 0
                                       ? "#f3a49c" : root.muted
                                font.pixelSize: 9
                                wrapMode: Text.WordWrap
                            }
                        }
                    }

                    GridLayout {
                        width: parent.width
                        columns: 2
                        columnSpacing: 14
                        rowSpacing: 9
                        visible: !gameDetails.loading

                        Text { visible: gameDetails.release_date.length > 0; text: "RELEASED"; color: "#687488"; font.pixelSize: 9; font.weight: Font.Bold }
                        Text { visible: gameDetails.release_date.length > 0; text: gameDetails.release_date; color: root.ink; font.pixelSize: 12; Layout.fillWidth: true; elide: Text.ElideRight }
                        Text { visible: gameDetails.developer.length > 0; text: "DEVELOPER"; color: "#687488"; font.pixelSize: 9; font.weight: Font.Bold }
                        Text { visible: gameDetails.developer.length > 0; text: gameDetails.developer; color: root.ink; font.pixelSize: 12; Layout.fillWidth: true; elide: Text.ElideRight }
                        Text { visible: gameDetails.publisher.length > 0; text: "PUBLISHER"; color: "#687488"; font.pixelSize: 9; font.weight: Font.Bold }
                        Text { visible: gameDetails.publisher.length > 0; text: gameDetails.publisher; color: root.ink; font.pixelSize: 12; Layout.fillWidth: true; elide: Text.ElideRight }
                        Text { visible: gameDetails.genre.length > 0; text: "GENRE"; color: "#687488"; font.pixelSize: 9; font.weight: Font.Bold }
                        Text { visible: gameDetails.genre.length > 0; text: gameDetails.genre; color: root.ink; font.pixelSize: 12; Layout.fillWidth: true; elide: Text.ElideRight }
                        Text { visible: gameDetails.players.length > 0; text: "PLAYERS"; color: "#687488"; font.pixelSize: 9; font.weight: Font.Bold }
                        Text { visible: gameDetails.players.length > 0; text: gameDetails.players; color: root.ink; font.pixelSize: 12; Layout.fillWidth: true; elide: Text.ElideRight }
                        Text { visible: gameDetails.rating.length > 0; text: "RATING"; color: "#687488"; font.pixelSize: 9; font.weight: Font.Bold }
                        Text { visible: gameDetails.rating.length > 0; text: gameDetails.rating + " / 5"; color: root.ink; font.pixelSize: 12; Layout.fillWidth: true; elide: Text.ElideRight }
                        Text { visible: gameDetails.esrb.length > 0; text: "RATED"; color: "#687488"; font.pixelSize: 9; font.weight: Font.Bold }
                        Text { visible: gameDetails.esrb.length > 0; text: gameDetails.esrb; color: root.ink; font.pixelSize: 12; Layout.fillWidth: true; elide: Text.ElideRight }
                        Text { visible: gameDetails.release_type.length > 0; text: "TYPE"; color: "#687488"; font.pixelSize: 9; font.weight: Font.Bold }
                        Text { visible: gameDetails.release_type.length > 0; text: gameDetails.release_type; color: root.ink; font.pixelSize: 12; Layout.fillWidth: true; elide: Text.ElideRight }
                    }

                    Column {
                        id: alternateTitleSection
                        width: parent.width
                        visible: !gameDetails.loading
                                 && gameDetails.alternate_title_count > 0
                        spacing: 8

                        Text {
                            text: "ALSO KNOWN AS"
                            color: "#687488"
                            font.pixelSize: 9
                            font.weight: Font.Bold
                            font.letterSpacing: 0.9
                        }

                        Flow {
                            id: alternateTitleFlow
                            width: parent.width
                            height: childrenRect.height
                            spacing: 6

                            Repeater {
                                model: gameDetails.alternate_title_count
                                delegate: Rectangle {
                                    id: alternateTitleChip
                                    required property int index
                                    property int revision: gameDetails.detail_revision
                                    property string aliasName: {
                                        alternateTitleChip.revision
                                        return gameDetails.alternate_title_at(
                                                    alternateTitleChip.index)
                                    }
                                    property string aliasRegion: {
                                        alternateTitleChip.revision
                                        return gameDetails.alternate_title_region_at(
                                                    alternateTitleChip.index)
                                    }
                                    width: Math.min(alternateTitleFlow.width,
                                                    alternateTitleText.implicitWidth + 18)
                                    height: 26
                                    radius: 7
                                    color: "#182532"
                                    border.color: "#31475b"

                                    Text {
                                        id: alternateTitleText
                                        anchors.left: parent.left
                                        anchors.right: parent.right
                                        anchors.verticalCenter: parent.verticalCenter
                                        anchors.leftMargin: 9
                                        anchors.rightMargin: 9
                                        text: alternateTitleChip.aliasRegion.length > 0
                                              ? alternateTitleChip.aliasName + " · "
                                                + alternateTitleChip.aliasRegion
                                              : alternateTitleChip.aliasName
                                        color: "#b8cfe2"
                                        font.pixelSize: 10
                                        elide: Text.ElideRight
                                    }
                                }
                            }
                        }
                    }

                    Text {
                        width: parent.width
                        visible: gameDetails.description.length > 0
                        text: gameDetails.description
                        color: "#c0c8d4"
                        font.pixelSize: 12
                        lineHeight: 1.35
                        wrapMode: Text.WordWrap
                    }

                    Text {
                        width: parent.width
                        visible: gameDetails.notes.length > 0
                        text: gameDetails.notes
                        color: root.muted
                        font.pixelSize: 10
                        lineHeight: 1.3
                        wrapMode: Text.WrapAnywhere
                    }

                    Rectangle {
                        visible: !gameDetails.loading
                                 && gameDetails.local_file_count > 0
                                 && !gameDetails.preparable
                                 && !gameDetails.prepared
                        width: parent.width
                        height: localPlayColumn.implicitHeight + 28
                        radius: 11
                        color: "#142429"
                        border.color: "#31504b"

                        Column {
                            id: localPlayColumn
                            anchors.left: parent.left
                            anchors.right: parent.right
                            anchors.top: parent.top
                            anchors.margins: 14
                            spacing: 9

                            Row {
                                width: parent.width
                                spacing: 8
                                Column {
                                    width: parent.width - localRefresh.width - 8
                                    spacing: 3
                                    Text {
                                        text: "PLAY LOCALLY"
                                        color: root.accentCool
                                        font.pixelSize: 10
                                        font.weight: Font.Bold
                                        font.letterSpacing: 1.1
                                    }
                                    Text {
                                        text: gameDetails.emulator_preference_scope === "game"
                                              ? "GAME DEFAULT"
                                              : gameDetails.emulator_preference_scope === "platform"
                                                ? "PLATFORM DEFAULT"
                                                : "AUTOMATIC SELECTION"
                                        color: root.muted
                                        font.pixelSize: 8
                                        font.weight: Font.Bold
                                        font.letterSpacing: 0.6
                                    }
                                }
                                Button {
                                    id: localRefresh
                                    width: 76
                                    height: 30
                                    text: "REFRESH"
                                    enabled: !gameDetails.launch_discovery_busy
                                             && !gameDetails.launch_busy
                                    font.pixelSize: 9
                                    font.weight: Font.Bold
                                    onClicked: gameDetails.refresh_emulators()
                                    background: Rectangle {
                                        radius: 7
                                        color: parent.down ? "#28364a" : "#202b3a"
                                        border.color: root.line
                                    }
                                    contentItem: Text {
                                        text: parent.text
                                        color: root.muted
                                        font: parent.font
                                        horizontalAlignment: Text.AlignHCenter
                                        verticalAlignment: Text.AlignVCenter
                                    }
                                }
                            }

                            Column {
                                width: parent.width
                                visible: gameDetails.local_file_count > 1
                                spacing: 4
                                Text {
                                    text: "GAME FILE"
                                    color: "#7f8b9e"
                                    font.pixelSize: 8
                                    font.weight: Font.Bold
                                }
                                ComboBox {
                                    id: localFilePicker
                                    width: parent.width
                                    height: 36
                                    model: gameDetails.local_file_count
                                    currentIndex: gameDetails.selected_local_file
                                    displayText: currentIndex >= 0
                                                 ? gameDetails.local_file_label_at(currentIndex)
                                                 : "Select a local file"
                                    onActivated: function(index) {
                                        gameDetails.select_local_file(index)
                                    }
                                    delegate: ItemDelegate {
                                        required property int index
                                        width: localFilePicker.width
                                        text: gameDetails.local_file_label_at(index)
                                        font.pixelSize: 10
                                        highlighted: localFilePicker.highlightedIndex === index
                                    }
                                    contentItem: Text {
                                        leftPadding: 10
                                        rightPadding: 28
                                        text: localFilePicker.displayText
                                        color: root.ink
                                        font.pixelSize: 10
                                        verticalAlignment: Text.AlignVCenter
                                        elide: Text.ElideMiddle
                                    }
                                    background: Rectangle {
                                        radius: 7
                                        color: "#111923"
                                        border.color: root.line
                                    }
                                }
                            }

                            Column {
                                width: parent.width
                                spacing: 4
                                Text {
                                    text: "EMULATOR"
                                    color: "#7f8b9e"
                                    font.pixelSize: 8
                                    font.weight: Font.Bold
                                }
                                ComboBox {
                                    id: localEmulatorPicker
                                    width: parent.width
                                    height: 38
                                    visible: gameDetails.emulator_option_count > 0
                                    enabled: !gameDetails.launch_discovery_busy
                                             && !gameDetails.launch_busy
                                    model: gameDetails.emulator_option_count
                                    currentIndex: gameDetails.selected_emulator_option
                                    displayText: currentIndex >= 0
                                                 ? gameDetails.emulator_option_label_at(currentIndex)
                                                 : "Select an emulator"
                                    onActivated: function(index) {
                                        gameDetails.select_emulator_option(index)
                                    }
                                    delegate: ItemDelegate {
                                        required property int index
                                        width: localEmulatorPicker.width
                                        text: gameDetails.emulator_option_label_at(index)
                                        font.pixelSize: 10
                                        highlighted: localEmulatorPicker.highlightedIndex === index
                                    }
                                    contentItem: Text {
                                        leftPadding: 10
                                        rightPadding: 28
                                        text: localEmulatorPicker.displayText
                                        color: root.ink
                                        font.pixelSize: 10
                                        font.weight: Font.Medium
                                        verticalAlignment: Text.AlignVCenter
                                        elide: Text.ElideRight
                                    }
                                    background: Rectangle {
                                        radius: 7
                                        color: "#111923"
                                        border.color: root.accentCool
                                    }
                                }
                                Text {
                                    width: parent.width
                                    visible: gameDetails.emulator_option_count === 0
                                    text: gameDetails.launch_discovery_busy
                                          ? "Detecting installed emulators…"
                                          : gameDetails.emulator_name
                                    color: root.muted
                                    font.pixelSize: 10
                                    wrapMode: Text.WordWrap
                                }
                                Text {
                                    width: parent.width
                                    visible: gameDetails.emulator_summary.length > 0
                                    text: gameDetails.emulator_summary
                                    color: root.muted
                                    font.pixelSize: 9
                                    elide: Text.ElideMiddle
                                }
                            }

                            Row {
                                width: parent.width
                                visible: gameDetails.emulator_option_count > 0
                                spacing: 6
                                Button {
                                    width: (parent.width - 12) / 3
                                    height: 30
                                    text: "THIS GAME"
                                    enabled: !gameDetails.launch_busy
                                    font.pixelSize: 8
                                    font.weight: Font.Bold
                                    onClicked: gameDetails.save_game_emulator_preference()
                                    background: Rectangle {
                                        radius: 7
                                        color: gameDetails.emulator_preference_scope === "game"
                                               ? "#1d493f" : "#202b3a"
                                        border.color: gameDetails.emulator_preference_scope === "game"
                                                      ? root.accentCool : root.line
                                    }
                                    contentItem: Text { text: parent.text; color: root.ink; font: parent.font; horizontalAlignment: Text.AlignHCenter; verticalAlignment: Text.AlignVCenter }
                                }
                                Button {
                                    width: (parent.width - 12) / 3
                                    height: 30
                                    text: "PLATFORM"
                                    enabled: !gameDetails.launch_busy
                                    font.pixelSize: 8
                                    font.weight: Font.Bold
                                    onClicked: gameDetails.save_platform_emulator_preference()
                                    background: Rectangle {
                                        radius: 7
                                        color: gameDetails.emulator_preference_scope === "platform"
                                               ? "#1d493f" : "#202b3a"
                                        border.color: gameDetails.emulator_preference_scope === "platform"
                                                      ? root.accentCool : root.line
                                    }
                                    contentItem: Text { text: parent.text; color: root.ink; font: parent.font; horizontalAlignment: Text.AlignHCenter; verticalAlignment: Text.AlignVCenter }
                                }
                                Button {
                                    width: (parent.width - 12) / 3
                                    height: 30
                                    text: "RESET"
                                    enabled: gameDetails.emulator_preference_scope.length > 0
                                    font.pixelSize: 8
                                    font.weight: Font.Bold
                                    onClicked: {
                                        if (gameDetails.emulator_preference_scope === "game")
                                            gameDetails.clear_game_emulator_preference()
                                        else if (gameDetails.emulator_preference_scope === "platform")
                                            gameDetails.clear_platform_emulator_preference()
                                    }
                                    background: Rectangle {
                                        radius: 7
                                        color: parent.enabled ? "#202b3a" : "#18212c"
                                        border.color: root.line
                                    }
                                    contentItem: Text { text: parent.text; color: parent.enabled ? root.muted : "#526071"; font: parent.font; horizontalAlignment: Text.AlignHCenter; verticalAlignment: Text.AlignVCenter }
                                }
                            }

                            Button {
                                width: parent.width
                                height: 32
                                visible: gameDetails.prepared
                                         || gameDetails.emulator_option_count > 0
                                text: gameDetails.launch_profile_open
                                      ? "HIDE LAUNCH COMMAND"
                                      : "CUSTOMIZE LAUNCH COMMAND"
                                enabled: !gameDetails.launch_busy
                                font.pixelSize: 8
                                font.weight: Font.Bold
                                onClicked: {
                                    if (gameDetails.launch_profile_open)
                                        gameDetails.close_launch_profile_editor()
                                    else
                                        gameDetails.open_launch_profile_editor()
                                }
                                background: Rectangle {
                                    radius: 7
                                    color: parent.down ? "#273447" : "#182331"
                                    border.color: gameDetails.launch_profile_open
                                                  ? root.accentCool : root.line
                                }
                                contentItem: Text {
                                    text: parent.text
                                    color: gameDetails.launch_profile_open
                                           ? root.accentCool : root.muted
                                    font: parent.font
                                    horizontalAlignment: Text.AlignHCenter
                                    verticalAlignment: Text.AlignVCenter
                                }
                            }

                            Rectangle {
                                id: launchProfileCard
                                width: parent.width
                                height: launchProfileColumn.implicitHeight + 24
                                visible: gameDetails.launch_profile_open
                                radius: 9
                                color: "#101a27"
                                border.color: "#2a4357"

                                Column {
                                    id: launchProfileColumn
                                    x: 12
                                    y: 12
                                    width: parent.width - 24
                                    spacing: 9

                                    Row {
                                        width: parent.width
                                        spacing: 8
                                        Column {
                                            width: parent.width - launchProfileClose.width - 8
                                            spacing: 2
                                            Text {
                                                text: "LAUNCH COMMAND"
                                                color: root.ink
                                                font.pixelSize: 10
                                                font.weight: Font.Bold
                                                font.letterSpacing: 0.5
                                            }
                                            Text {
                                                width: parent.width
                                                text: "Exact argv customization for "
                                                      + gameDetails.emulator_name
                                                color: root.muted
                                                font.pixelSize: 8
                                                elide: Text.ElideRight
                                            }
                                        }
                                        Button {
                                            id: launchProfileClose
                                            width: 28
                                            height: 26
                                            text: "×"
                                            onClicked: gameDetails.close_launch_profile_editor()
                                            background: Rectangle {
                                                radius: 6
                                                color: parent.down ? "#2b3747" : "#1b2735"
                                                border.color: root.line
                                            }
                                        }
                                    }

                                    Column {
                                        width: parent.width
                                        spacing: 4
                                        Text {
                                            text: "SAVE SCOPE"
                                            color: "#7f8b9e"
                                            font.pixelSize: 8
                                            font.weight: Font.Bold
                                        }
                                        ComboBox {
                                            id: launchProfileScope
                                            width: parent.width
                                            height: 34
                                            model: ["This game", "This platform", "All platforms"]
                                            currentIndex: gameDetails.launch_profile_scope === "platform"
                                                          ? 1
                                                          : gameDetails.launch_profile_scope === "global"
                                                            ? 2 : 0
                                            onActivated: function(index) {
                                                gameDetails.select_launch_profile_scope(
                                                    index === 1 ? "platform"
                                                                : index === 2 ? "global" : "game")
                                            }
                                            background: Rectangle {
                                                radius: 7
                                                color: "#111923"
                                                border.color: root.line
                                            }
                                        }
                                    }

                                    Rectangle {
                                        width: parent.width
                                        height: defaultLaunchTemplate.implicitHeight + 20
                                        radius: 7
                                        color: "#0c141f"
                                        border.color: "#223246"
                                        Text {
                                            id: defaultLaunchTemplate
                                            x: 10
                                            y: 10
                                            width: parent.width - 20
                                            text: "BUILT-IN  "
                                                  + gameDetails.launch_profile_default_template
                                            color: "#94a2b5"
                                            font.family: "monospace"
                                            font.pixelSize: 9
                                            wrapMode: Text.WrapAnywhere
                                        }
                                    }

                                    Text {
                                        width: parent.width
                                        text: gameDetails.launch_profile_inheritance
                                        color: "#75c8ba"
                                        font.pixelSize: 8
                                        wrapMode: Text.WordWrap
                                    }

                                    Column {
                                        width: parent.width
                                        spacing: 4
                                        Text {
                                            text: "EXTRA ARGUMENTS"
                                            color: "#7f8b9e"
                                            font.pixelSize: 8
                                            font.weight: Font.Bold
                                        }
                                        TextField {
                                            id: launchExtraArguments
                                            width: parent.width
                                            height: 34
                                            text: gameDetails.launch_profile_extra_arguments
                                            placeholderText: "Example: --fullscreen --latency 1"
                                            selectByMouse: true
                                            font.family: "monospace"
                                            font.pixelSize: 9
                                            background: Rectangle {
                                                radius: 7
                                                color: "#0b121b"
                                                border.color: parent.activeFocus
                                                              ? root.accentCool : root.line
                                            }
                                        }
                                    }

                                    Column {
                                        width: parent.width
                                        spacing: 4
                                        Text {
                                            text: "COMMAND TEMPLATE (OPTIONAL)"
                                            color: "#7f8b9e"
                                            font.pixelSize: 8
                                            font.weight: Font.Bold
                                        }
                                        ScrollView {
                                            width: parent.width
                                            height: 74
                                            clip: true
                                            TextArea {
                                                id: launchCommandTemplate
                                                text: gameDetails.launch_profile_command_template
                                                placeholderText: "Leave blank for the built-in template"
                                                selectByMouse: true
                                                wrapMode: TextEdit.WrapAnywhere
                                                font.family: "monospace"
                                                font.pixelSize: 9
                                                color: root.ink
                                                background: Rectangle {
                                                    radius: 7
                                                    color: "#0b121b"
                                                    border.color: parent.activeFocus
                                                                  ? root.accentCool : root.line
                                                }
                                            }
                                        }
                                    }

                                    Text {
                                        width: parent.width
                                        text: "Use the placeholders shown by the built-in template; %f is the selected file and %% is a literal percent. Prepared PC games may expose %{config}, %{shared_config}, %{game_path}, %{game_id}, or %{vm_root}. Quotes only group arguments; Lunchbox never invokes a shell. A template replaces the built-in argv, while extra arguments augment it when the template is blank."
                                        color: root.muted
                                        font.pixelSize: 8
                                        lineHeight: 1.2
                                        wrapMode: Text.WordWrap
                                    }

                                    Text {
                                        width: parent.width
                                        visible: gameDetails.launch_profile_status.length > 0
                                        text: gameDetails.launch_profile_status
                                        color: gameDetails.launch_profile_status.indexOf("Could not") === 0
                                               ? "#ef9999" : "#9bd7cc"
                                        font.pixelSize: 8
                                        wrapMode: Text.WordWrap
                                    }

                                    Row {
                                        width: parent.width
                                        spacing: 7
                                        Button {
                                            width: (parent.width - 7) * 0.62
                                            height: 32
                                            text: "SAVE PROFILE"
                                            font.pixelSize: 8
                                            font.weight: Font.Bold
                                            onClicked: gameDetails.save_launch_profile(
                                                           launchExtraArguments.text,
                                                           launchCommandTemplate.text)
                                            background: Rectangle {
                                                radius: 7
                                                color: parent.down ? "#93601e" : "#b77a2c"
                                                border.color: "#d99a48"
                                            }
                                        }
                                        Button {
                                            width: (parent.width - 7) * 0.38
                                            height: 32
                                            text: "CLEAR SCOPE"
                                            font.pixelSize: 8
                                            font.weight: Font.Bold
                                            onClicked: gameDetails.clear_launch_profile()
                                            background: Rectangle {
                                                radius: 7
                                                color: parent.down ? "#39252a" : "#271d25"
                                                border.color: "#60404b"
                                            }
                                        }
                                    }
                                }
                            }

                            Rectangle {
                                width: parent.width
                                height: firmwareColumn.implicitHeight + 24
                                visible: gameDetails.firmware_rule_count > 0
                                radius: 9
                                color: gameDetails.firmware_missing_count > 0
                                       || gameDetails.firmware_manual_count > 0
                                       ? "#321f24" : "#142d2a"
                                border.color: gameDetails.firmware_missing_count > 0
                                              || gameDetails.firmware_manual_count > 0
                                              ? "#7a3c48" : "#315c54"

                                Column {
                                    id: firmwareColumn
                                    anchors.left: parent.left
                                    anchors.right: parent.right
                                    anchors.leftMargin: 12
                                    anchors.rightMargin: 12
                                    anchors.verticalCenter: parent.verticalCenter
                                    spacing: 5
                                    Row {
                                        width: parent.width
                                        spacing: 8
                                        Text {
                                            width: parent.width - firmwareFolderButton.width - 8
                                            text: gameDetails.firmware_manual_count > 0
                                                  ? "MANUAL FIRMWARE REQUIRED"
                                                  : gameDetails.firmware_missing_count > 0
                                                    ? "FIRMWARE REQUIRED"
                                                    : gameDetails.firmware_optional_count > 0
                                                      ? "OPTIONAL FIRMWARE AVAILABLE"
                                                    : "FIRMWARE READY"
                                            color: gameDetails.firmware_missing_count > 0
                                                   || gameDetails.firmware_manual_count > 0
                                                   ? "#ff9b91" : root.accentCool
                                            font.pixelSize: 9
                                            font.weight: Font.Bold
                                            font.letterSpacing: 0.7
                                            verticalAlignment: Text.AlignVCenter
                                        }
                                        Button {
                                            id: firmwareFolderButton
                                            visible: gameDetails.firmware_runtime_path.length > 0
                                            width: visible ? 86 : 0
                                            height: 26
                                            text: "OPEN FOLDER"
                                            font.pixelSize: 8
                                            font.weight: Font.Bold
                                            onClicked: gameDetails.open_firmware_directory()
                                            background: Rectangle {
                                                radius: 6
                                                color: parent.down ? "#2a394c" : "#202b3a"
                                                border.color: root.line
                                            }
                                            contentItem: Text {
                                                text: parent.text
                                                color: root.ink
                                                font: parent.font
                                                horizontalAlignment: Text.AlignHCenter
                                                verticalAlignment: Text.AlignVCenter
                                            }
                                        }
                                    }
                                    Text {
                                        width: parent.width
                                        text: gameDetails.firmware_summary
                                        color: root.ink
                                        font.pixelSize: 10
                                        wrapMode: Text.WordWrap
                                    }
                                    Text {
                                        width: parent.width
                                        text: gameDetails.firmware_package_summary
                                        color: root.muted
                                        font.pixelSize: 9
                                        elide: Text.ElideMiddle
                                    }
                                    Text {
                                        width: parent.width
                                        text: gameDetails.firmware_source_summary
                                              + (gameDetails.firmware_runtime_path.length > 0
                                                 ? " · " + gameDetails.firmware_runtime_path : "")
                                        color: root.muted
                                        font.pixelSize: 8
                                        elide: Text.ElideMiddle
                                    }
                                    Button {
                                        width: parent.width
                                        height: 30
                                        visible: gameDetails.firmware_can_download
                                                 || gameDetails.firmware_can_sync
                                        enabled: !gameDetails.firmware_busy
                                        text: gameDetails.firmware_busy ? "VERIFYING…"
                                              : gameDetails.firmware_can_download
                                                ? "DOWNLOAD & INSTALL"
                                                : "SYNC / REPAIR"
                                        font.pixelSize: 9
                                        font.weight: Font.Bold
                                        onClicked: {
                                            if (gameDetails.firmware_can_download)
                                                gameDetails.download_firmware()
                                            else
                                                gameDetails.sync_firmware()
                                        }
                                        background: Rectangle {
                                            radius: 6
                                            color: parent.enabled
                                                   ? (parent.down ? "#315047" : "#203a35")
                                                   : "#18212c"
                                            border.color: parent.enabled ? root.accentCool : root.line
                                        }
                                        contentItem: Text {
                                            text: parent.text
                                            color: parent.enabled ? root.accentCool : root.muted
                                            font: parent.font
                                            horizontalAlignment: Text.AlignHCenter
                                            verticalAlignment: Text.AlignVCenter
                                        }
                                    }
                                    Button {
                                        width: parent.width
                                        height: 28
                                        visible: gameDetails.firmware_needs_import
                                        enabled: !gameDetails.firmware_busy
                                        text: "IMPORT A LOCAL COPY INSTEAD"
                                        font.pixelSize: 8
                                        font.weight: Font.Bold
                                        onClicked: firmwarePackageDialog.open()
                                        background: Rectangle {
                                            radius: 6
                                            color: parent.down ? "#2a394c" : "#202b3a"
                                            border.color: root.line
                                        }
                                        contentItem: Text {
                                            text: parent.text
                                            color: parent.enabled ? root.muted : "#526071"
                                            font: parent.font
                                            horizontalAlignment: Text.AlignHCenter
                                            verticalAlignment: Text.AlignVCenter
                                        }
                                    }
                                }
                            }

                            Text {
                                width: parent.width
                                visible: gameDetails.launch_status.length > 0
                                text: gameDetails.launch_status
                                color: gameDetails.can_launch ? "#a8d9cf" : root.muted
                                font.pixelSize: 10
                                lineHeight: 1.25
                                wrapMode: Text.WordWrap
                            }

                            Button {
                                width: parent.width
                                height: 44
                                text: gameDetails.launch_busy ? "STARTING…"
                                      : gameDetails.game_running ? "GAME IS RUNNING"
                                      : "PLAY"
                                enabled: gameDetails.can_launch
                                         && !gameDetails.launch_busy
                                         && !gameDetails.game_running
                                         && !gameDetails.firmware_busy
                                font.pixelSize: 11
                                font.weight: Font.Bold
                                onClicked: gameDetails.launch_game()
                                background: Rectangle {
                                    radius: 8
                                    color: parent.enabled
                                           ? (parent.down ? "#d24e36" : root.accent)
                                           : "#26313e"
                                    border.color: parent.enabled ? "#ff8a70" : root.line
                                }
                                contentItem: Text {
                                    text: parent.text
                                    color: parent.enabled ? "#ffffff" : root.muted
                                    font: parent.font
                                    horizontalAlignment: Text.AlignHCenter
                                    verticalAlignment: Text.AlignVCenter
                                }
                            }
                        }
                    }

                    Rectangle {
                        visible: !gameDetails.loading
                                 && (gameDetails.preparable || gameDetails.prepared
                                     || gameDetails.prepare_busy)
                        width: parent.width
                        height: prepareColumn.implicitHeight + 28
                        radius: 11
                        color: gameDetails.prepared ? "#142d2a" : "#171f2b"
                        border.color: gameDetails.prepared ? root.accentCool : "#3b4658"

                        Column {
                            id: prepareColumn
                            anchors.left: parent.left
                            anchors.right: parent.right
                            anchors.top: parent.top
                            anchors.margins: 14
                            spacing: 9

                            Row {
                                width: parent.width
                                spacing: 8
                                Text {
                                    width: parent.width - prepareState.width - 8
                                    text: "PC INSTALL"
                                    color: gameDetails.prepared ? root.accentCool : root.accent
                                    font.pixelSize: 10
                                    font.weight: Font.Bold
                                    font.letterSpacing: 1.1
                                }
                                Rectangle {
                                    id: prepareState
                                    width: prepareStateText.implicitWidth + 14
                                    height: 22
                                    radius: 7
                                    color: gameDetails.prepared ? "#1d493f" : "#252f3e"
                                    border.color: gameDetails.prepared ? root.accentCool : root.line
                                    Text {
                                        id: prepareStateText
                                        anchors.centerIn: parent
                                        text: gameDetails.prepare_busy ? "WORKING"
                                              : gameDetails.prepared ? "READY" : "ARCHIVED"
                                        color: gameDetails.prepared ? root.accentCool : root.muted
                                        font.pixelSize: 8
                                        font.weight: Font.Bold
                                        font.letterSpacing: 0.7
                                    }
                                }
                            }
                            Text {
                                width: parent.width
                                text: gameDetails.prepared
                                      ? "The game and its exact launch metadata are available in Lunchbox's versioned prepared cache."
                                      : "Unpack this eXo archive with its matching metadata and shared runtime assets into a reusable, private cache."
                                color: "#b8c2d0"
                                font.pixelSize: 11
                                lineHeight: 1.25
                                wrapMode: Text.WordWrap
                            }
                            Text {
                                width: parent.width
                                visible: gameDetails.prepared_summary.length > 0
                                         || gameDetails.preparation_file.length > 0
                                text: gameDetails.prepared_summary.length > 0
                                      ? gameDetails.prepared_summary
                                      : gameDetails.preparation_file
                                color: root.muted
                                font.pixelSize: 9
                                wrapMode: Text.WrapAnywhere
                                maximumLineCount: 3
                                elide: Text.ElideMiddle
                            }
                            Rectangle {
                                id: prepareProgress
                                width: parent.width
                                height: 4
                                visible: gameDetails.prepare_busy
                                radius: 2
                                color: "#263246"
                                clip: true
                                Rectangle {
                                    id: prepareProgressThumb
                                    width: Math.max(48, prepareProgress.width * 0.32)
                                    height: parent.height
                                    radius: parent.radius
                                    color: root.accentCool
                                    SequentialAnimation on x {
                                        running: gameDetails.prepare_busy
                                        loops: Animation.Infinite
                                        NumberAnimation {
                                            from: -prepareProgressThumb.width
                                            to: prepareProgress.width
                                            duration: 900
                                            easing.type: Easing.InOutCubic
                                        }
                                    }
                                }
                            }
                            Rectangle {
                                width: parent.width
                                height: 1
                                visible: gameDetails.prepared
                                color: "#31504b"
                            }
                            Row {
                                width: parent.width
                                visible: gameDetails.prepared
                                spacing: 8
                                Column {
                                    width: parent.width - refreshEmulators.width - 8
                                    spacing: 3
                                    Text {
                                        width: parent.width
                                        text: gameDetails.launch_discovery_busy
                                              ? "DETECTING EMULATOR…"
                                              : gameDetails.emulator_name.length > 0
                                                ? gameDetails.emulator_name.toUpperCase()
                                                : "EMULATOR"
                                        color: gameDetails.can_launch ? root.ink : root.muted
                                        font.pixelSize: 10
                                        font.weight: Font.Bold
                                        elide: Text.ElideRight
                                    }
                                    Text {
                                        width: parent.width
                                        visible: gameDetails.emulator_summary.length > 0
                                        text: gameDetails.emulator_summary
                                        color: root.muted
                                        font.pixelSize: 9
                                        elide: Text.ElideMiddle
                                    }
                                }
                                Button {
                                    id: refreshEmulators
                                    width: 76
                                    height: 30
                                    text: "REFRESH"
                                    enabled: !gameDetails.launch_discovery_busy
                                             && !gameDetails.launch_busy
                                    font.pixelSize: 9
                                    font.weight: Font.Bold
                                    onClicked: gameDetails.refresh_emulators()
                                    background: Rectangle {
                                        radius: 7
                                        color: parent.down ? "#28364a" : "#202b3a"
                                        border.color: root.line
                                    }
                                    contentItem: Text {
                                        text: parent.text
                                        color: root.muted
                                        font: parent.font
                                        horizontalAlignment: Text.AlignHCenter
                                        verticalAlignment: Text.AlignVCenter
                                    }
                                }
                            }
                            Text {
                                width: parent.width
                                visible: gameDetails.prepared
                                         && gameDetails.launch_status.length > 0
                                text: gameDetails.launch_status
                                color: gameDetails.can_launch ? "#a8d9cf" : root.muted
                                font.pixelSize: 10
                                lineHeight: 1.25
                                wrapMode: Text.WordWrap
                            }
                            Button {
                                width: parent.width
                                height: 42
                                visible: gameDetails.prepared
                                text: gameDetails.launch_busy ? "STARTING…"
                                      : gameDetails.game_running ? "GAME IS RUNNING"
                                      : "PLAY"
                                enabled: gameDetails.can_launch
                                         && !gameDetails.launch_busy
                                         && !gameDetails.game_running
                                         && !gameDetails.prepare_busy
                                font.pixelSize: 11
                                font.weight: Font.Bold
                                onClicked: gameDetails.launch_game()
                                background: Rectangle {
                                    radius: 8
                                    color: parent.enabled
                                           ? (parent.down ? "#d24e36" : root.accent)
                                           : "#26313e"
                                    border.color: parent.enabled ? "#ff8a70" : root.line
                                }
                                contentItem: Text {
                                    text: parent.text
                                    color: parent.enabled ? "#ffffff" : root.muted
                                    font: parent.font
                                    horizontalAlignment: Text.AlignHCenter
                                    verticalAlignment: Text.AlignVCenter
                                }
                            }
                            Button {
                                width: parent.width
                                height: 36
                                text: gameDetails.prepare_busy ? "CANCEL PREPARATION"
                                      : gameDetails.prepared ? "VERIFY & REFRESH INSTALL"
                                      : "PREPARE INSTALL"
                                enabled: gameDetails.preparable || gameDetails.prepare_busy
                                font.pixelSize: 10
                                font.weight: Font.Bold
                                onClicked: {
                                    if (gameDetails.prepare_busy)
                                        gameDetails.cancel_preparation()
                                    else
                                        gameDetails.prepare_game()
                                }
                                background: Rectangle {
                                    radius: 8
                                    color: parent.down ? "#28364a"
                                           : gameDetails.prepared ? "#1d493f" : "#263246"
                                    border.color: gameDetails.prepared ? root.accentCool : root.accent
                                }
                                contentItem: Text {
                                    text: parent.text
                                    color: gameDetails.prepared ? root.accentCool : root.ink
                                    font: parent.font
                                    horizontalAlignment: Text.AlignHCenter
                                    verticalAlignment: Text.AlignVCenter
                                }
                            }
                        }
                    }

                    Rectangle {
                        visible: !gameDetails.loading && gameDetails.bundle_count > 0
                        width: parent.width
                        height: 1
                        color: root.line
                    }
                    Text {
                        visible: !gameDetails.loading && gameDetails.bundle_count > 0
                        width: parent.width
                        text: "MINERVA SOURCES"
                        color: "#687488"
                        font.pixelSize: 10
                        font.weight: Font.Bold
                        font.letterSpacing: 1.2
                    }
                    Text {
                        visible: !gameDetails.loading && gameDetails.bundle_count > 0
                        width: parent.width
                        text: "Choose a verified platform bundle, then inspect candidates ranked by title, your preferred region, and release version before adding anything to the download queue."
                        color: root.muted
                        font.pixelSize: 11
                        lineHeight: 1.25
                        wrapMode: Text.WordWrap
                    }

                    Repeater {
                        model: gameDetails.bundle_count
                        delegate: Rectangle {
                            id: sourceRow
                            required property int index
                            property int revision: gameDetails.detail_revision
                            width: detailsPane.width - 40
                            height: 66
                            radius: 9
                            color: gameDetails.selected_bundle === index ? "#2a2e32" : sourceHover.hovered ? "#202a39" : root.panelRaised
                            border.color: gameDetails.selected_bundle === index ? root.accent : root.line
                            HoverHandler { id: sourceHover }
                            TapHandler { onTapped: gameDetails.load_bundle_files(sourceRow.index) }
                            Column {
                                anchors.left: parent.left
                                anchors.right: parent.right
                                anchors.leftMargin: 12
                                anchors.rightMargin: 12
                                anchors.verticalCenter: parent.verticalCenter
                                spacing: 4
                                Text {
                                    width: parent.width
                                    text: { sourceRow.revision; return gameDetails.bundle_title_at(sourceRow.index) }
                                    color: root.ink
                                    font.pixelSize: 12
                                    font.weight: Font.DemiBold
                                    elide: Text.ElideRight
                                }
                                Text {
                                    width: parent.width
                                    text: { sourceRow.revision; return gameDetails.bundle_detail_at(sourceRow.index) }
                                    color: root.muted
                                    font.pixelSize: 10
                                    elide: Text.ElideRight
                                }
                            }
                        }
                    }

                    Text {
                        id: fileCandidateHeading
                        visible: gameDetails.file_count > 0
                        width: parent.width
                        topPadding: 6
                        text: "TITLE CANDIDATES — REVIEW REQUIRED"
                        color: root.accent
                        font.pixelSize: 10
                        font.weight: Font.Bold
                        font.letterSpacing: 0.8
                    }
                    Repeater {
                        model: gameDetails.file_count
                        delegate: Rectangle {
                            id: fileRow
                            required property int index
                            property int revision: gameDetails.detail_revision
                            width: detailsPane.width - 40
                            height: Math.max(68, fileName.implicitHeight + fileInfo.implicitHeight + 24)
                            radius: 8
                            color: fileRow.index === 0 ? "#17272a" : "#151d29"
                            border.color: fileRow.index === 0 ? root.accentCool : root.line
                            Column {
                                anchors.left: parent.left
                                anchors.right: getButton.left
                                anchors.leftMargin: 11
                                anchors.rightMargin: 9
                                anchors.verticalCenter: parent.verticalCenter
                                spacing: 4
                                Text {
                                    id: fileName
                                    width: parent.width
                                    text: { fileRow.revision; return gameDetails.file_name_at(fileRow.index) }
                                    color: root.ink
                                    font.pixelSize: 11
                                    wrapMode: Text.WrapAnywhere
                                }
                                Text {
                                    id: fileInfo
                                    width: parent.width
                                    text: { fileRow.revision; return gameDetails.file_detail_at(fileRow.index) }
                                    color: root.muted
                                    font.pixelSize: 10
                                    elide: Text.ElideRight
                                }
                            }
                            HeaderButton {
                                id: getButton
                                anchors.right: parent.right
                                anchors.rightMargin: 9
                                anchors.verticalCenter: parent.verticalCenter
                                text: gameDetails.download_busy ? "…"
                                      : gameDetails.file_has_download_plan(fileRow.index)
                                        ? "GET SET" : "GET"
                                enabled: !gameDetails.download_busy && !gameDetails.torrent_loading
                                implicitWidth: 62
                                implicitHeight: 34
                                leftPadding: 8
                                rightPadding: 8
                                onClicked: {
                                    if (gameDetails.file_has_download_plan(fileRow.index)) {
                                        root.pendingDownloadPlanIndex = fileRow.index
                                        downloadPlanDialog.open()
                                    } else {
                                        gameDetails.queue_file(fileRow.index)
                                    }
                                }
                            }
                        }
                    }

                    Item { width: 1; height: 10 }
                }
            }
        }
    }

    Dialog {
        id: downloadPlanDialog
        readonly property string planKind: root.pendingDownloadPlanIndex >= 0
                                           ? gameDetails.file_plan_kind_at(root.pendingDownloadPlanIndex)
                                           : ""
        modal: true
        anchors.centerIn: parent
        width: Math.min(760, root.width - 48)
        height: Math.min(planKind === "arcade_mame_laserdisc" ? 430 : 620,
                         root.height - 48)
        padding: 20
        closePolicy: Popup.CloseOnEscape
        onClosed: root.pendingDownloadPlanIndex = -1

        background: Rectangle {
            color: root.panel
            radius: 14
            border.color: root.accentCool
        }
        header: Rectangle {
            width: parent.width
            height: 68
            color: "#17272a"
            radius: 14
            Column {
                anchors.left: parent.left
                anchors.leftMargin: 22
                anchors.verticalCenter: parent.verticalCenter
                spacing: 3
                Text {
                    text: downloadPlanDialog.planKind === "optical_multidisc"
                          ? "REVIEW MULTI-DISC DOWNLOAD"
                          : downloadPlanDialog.planKind.indexOf("arcade_") === 0
                            ? "REVIEW ARCADE MACHINE DOWNLOAD"
                            : "REVIEW RELATED DOWNLOAD"
                    color: root.ink
                    font.pixelSize: 17
                    font.weight: Font.Bold
                    font.letterSpacing: 0.8
                }
                Text {
                    text: root.pendingDownloadPlanIndex >= 0
                          ? gameDetails.file_plan_summary_at(root.pendingDownloadPlanIndex) : ""
                    color: root.accentCool
                    font.pixelSize: 10
                }
            }
        }
        contentItem: ColumnLayout {
            spacing: 12
            Text {
                Layout.fillWidth: true
                text: appSettings.download_entire_torrent
                      ? "Whole-torrent mode is enabled. Lunchbox will download the complete source bundle; switch it off in Settings to select only the required set below."
                      : downloadPlanDialog.planKind === "optical_multidisc"
                        ? "Lunchbox will select every required disc and companion file as one queue item, preserve their relative layout, and publish the M3U playlist only after the complete set is safely imported."
                        : downloadPlanDialog.planKind === "arcade_mame_laserdisc"
                          ? "Lunchbox will select the exact MAME ROM set and laserdisc CHD listed below. After both arrive, it stages the native set-name layout and then marks the game installed."
                          : downloadPlanDialog.planKind.indexOf("arcade_") === 0
                          ? "Lunchbox will select only the exact ROM, framefile, machine data, video, and audio members listed below. After every member arrives, it stages the native MAME or Hypseus-compatible layout and then marks the game installed."
                          : "Lunchbox will select the exact eXo game, metadata, game-data, and utility archives available for this title as one queue item. Shared dependencies are retained in a per-source cache for later prepared installation."
                color: appSettings.download_entire_torrent ? root.accent : root.muted
                font.pixelSize: 11
                wrapMode: Text.WordWrap
            }
            Rectangle { Layout.fillWidth: true; Layout.preferredHeight: 1; color: root.line }
            Text {
                text: "EXACT TORRENT MEMBERS"
                color: root.accent
                font.pixelSize: 10
                font.weight: Font.Bold
                font.letterSpacing: 1.2
            }
            ScrollView {
                Layout.fillWidth: true
                Layout.fillHeight: true
                clip: true
                Text {
                    width: downloadPlanDialog.availableWidth - 40
                    text: root.pendingDownloadPlanIndex >= 0
                          ? gameDetails.file_plan_members_at(root.pendingDownloadPlanIndex) : ""
                    color: root.ink
                    font.family: "monospace"
                    font.pixelSize: 10
                    lineHeight: 1.35
                    wrapMode: Text.WrapAnywhere
                }
            }
            Text {
                Layout.fillWidth: true
                text: "No filename match is treated as identity. The exact selection above remains visible for review before qBittorrent is changed."
                color: root.muted
                font.pixelSize: 10
                wrapMode: Text.WordWrap
            }
        }
        footer: Rectangle {
            width: parent.width
            height: 66
            color: root.panelRaised
            radius: 14
            Row {
                anchors.right: parent.right
                anchors.rightMargin: 18
                anchors.verticalCenter: parent.verticalCenter
                spacing: 9
                Button {
                    text: "Cancel"
                    enabled: !gameDetails.download_busy
                    onClicked: downloadPlanDialog.close()
                }
                Button {
                    text: appSettings.download_entire_torrent
                          ? "Download whole torrent"
                          : downloadPlanDialog.planKind.indexOf("arcade_") === 0
                            ? "Download layout" : "Download set"
                    highlighted: true
                    enabled: root.pendingDownloadPlanIndex >= 0
                             && !gameDetails.download_busy
                    onClicked: {
                        const index = root.pendingDownloadPlanIndex
                        downloadPlanDialog.close()
                        gameDetails.queue_file(index)
                    }
                }
            }
        }
    }

    Dialog {
        id: collectionEditorDialog
        modal: true
        anchors.centerIn: parent
        width: Math.min(520, root.width - 48)
        height: 390
        padding: 22
        closePolicy: Popup.CloseOnEscape

        background: Rectangle {
            color: root.panel
            radius: 14
            border.color: root.line
        }
        header: Rectangle {
            width: parent.width
            height: 62
            color: root.panelRaised
            radius: 14
            Text {
                anchors.left: parent.left
                anchors.leftMargin: 22
                anchors.verticalCenter: parent.verticalCenter
                text: root.editingCollectionId.length > 0
                      ? "EDIT COLLECTION" : "NEW COLLECTION"
                color: root.ink
                font.pixelSize: 17
                font.weight: Font.Bold
                font.letterSpacing: 0.8
            }
        }
        contentItem: ColumnLayout {
            spacing: 10
            Text {
                text: "NAME"
                color: root.accent
                font.pixelSize: 10
                font.weight: Font.Bold
                font.letterSpacing: 1.2
            }
            TextField {
                id: collectionNameField
                Layout.fillWidth: true
                maximumLength: 100
                placeholderText: "For example: Couch co-op"
                onAccepted: {
                    if (text.trim().length > 0 && !library.collection_busy)
                        root.saveEditedCollection()
                }
            }
            Text {
                text: "DESCRIPTION"
                color: root.accent
                font.pixelSize: 10
                font.weight: Font.Bold
                font.letterSpacing: 1.2
            }
            ScrollView {
                Layout.fillWidth: true
                Layout.fillHeight: true
                TextArea {
                    id: collectionDescriptionField
                    placeholderText: "Optional notes about this collection"
                    wrapMode: TextEdit.Wrap
                    selectByMouse: true
                }
            }
            Text {
                Layout.fillWidth: true
                text: library.collection_busy ? library.collection_message
                      : "Collections are stored in your local Lunchbox profile."
                color: root.muted
                font.pixelSize: 10
                wrapMode: Text.WordWrap
            }
        }
        footer: Rectangle {
            width: parent.width
            height: 64
            color: root.panelRaised
            radius: 14
            Row {
                anchors.right: parent.right
                anchors.rightMargin: 18
                anchors.verticalCenter: parent.verticalCenter
                spacing: 9
                Button {
                    text: "Cancel"
                    enabled: !library.collection_busy
                    onClicked: collectionEditorDialog.close()
                }
                Button {
                    id: saveCollectionButton
                    text: root.editingCollectionId.length > 0 ? "Save" : "Create"
                    highlighted: true
                    enabled: collectionNameField.text.trim().length > 0
                             && !library.collection_busy
                    onClicked: root.saveEditedCollection()
                }
            }
        }
    }

    Dialog {
        id: manageCollectionsDialog
        modal: true
        anchors.centerIn: parent
        width: Math.min(680, root.width - 48)
        height: Math.min(610, root.height - 48)
        padding: 18
        closePolicy: Popup.CloseOnEscape
        onOpened: {
            if (managedCollectionList.currentIndex < 0
                    && library.collection_count > 0)
                managedCollectionList.currentIndex = 0
        }

        background: Rectangle {
            color: root.panel
            radius: 14
            border.color: root.line
        }
        header: Rectangle {
            width: parent.width
            height: 62
            color: root.panelRaised
            radius: 14
            Text {
                anchors.left: parent.left
                anchors.leftMargin: 22
                anchors.verticalCenter: parent.verticalCenter
                text: "MANAGE COLLECTIONS"
                color: root.ink
                font.pixelSize: 17
                font.weight: Font.Bold
                font.letterSpacing: 0.8
            }
            RoundButton {
                anchors.right: parent.right
                anchors.rightMargin: 14
                anchors.verticalCenter: parent.verticalCenter
                text: "×"
                flat: true
                font.pixelSize: 20
                onClicked: manageCollectionsDialog.close()
            }
        }
        contentItem: ColumnLayout {
            spacing: 12
            Text {
                Layout.fillWidth: true
                text: "Build focused shelves without changing your ROM library. Collection filters can be combined with search and platform navigation."
                color: root.muted
                font.pixelSize: 11
                wrapMode: Text.WordWrap
            }
            ListView {
                id: managedCollectionList
                Layout.fillWidth: true
                Layout.fillHeight: true
                clip: true
                spacing: 6
                model: library.collection_count
                currentIndex: library.collection_count > 0 ? 0 : -1
                ScrollBar.vertical: ScrollBar { policy: ScrollBar.AsNeeded }
                delegate: Rectangle {
                    id: managedCollectionRow
                    required property int index
                    property int revision: library.collection_revision
                    width: ListView.view.width
                    height: 72
                    radius: 9
                    color: ListView.isCurrentItem ? "#272c34"
                           : managedHover.hovered ? "#1b2330" : "#111720"
                    border.color: ListView.isCurrentItem ? root.accent : root.line
                    HoverHandler { id: managedHover }
                    TapHandler {
                        onTapped: managedCollectionList.currentIndex = managedCollectionRow.index
                        onDoubleTapped: root.openEditCollectionDialog(managedCollectionRow.index)
                    }
                    Column {
                        anchors.left: parent.left
                        anchors.right: countText.left
                        anchors.leftMargin: 15
                        anchors.rightMargin: 12
                        anchors.verticalCenter: parent.verticalCenter
                        spacing: 4
                        Text {
                            width: parent.width
                            text: {
                                managedCollectionRow.revision
                                return library.collection_name_at(managedCollectionRow.index)
                            }
                            color: root.ink
                            font.pixelSize: 14
                            font.weight: Font.DemiBold
                            elide: Text.ElideRight
                        }
                        Text {
                            width: parent.width
                            text: {
                                managedCollectionRow.revision
                                const description = library.collection_description_at(
                                                    managedCollectionRow.index)
                                return description.length > 0 ? description : "No description"
                            }
                            color: root.muted
                            font.pixelSize: 10
                            elide: Text.ElideRight
                        }
                    }
                    Text {
                        id: countText
                        anchors.right: parent.right
                        anchors.rightMargin: 15
                        anchors.verticalCenter: parent.verticalCenter
                        text: library.collection_game_count_at(managedCollectionRow.index)
                        color: root.accent
                        font.pixelSize: 15
                        font.weight: Font.Bold
                    }
                }
            }
            Text {
                Layout.fillWidth: true
                visible: library.collection_count === 0
                text: "No collections yet. Create one, then add games from the details panel."
                color: root.muted
                horizontalAlignment: Text.AlignHCenter
                wrapMode: Text.WordWrap
            }
            Text {
                Layout.fillWidth: true
                text: library.collection_message
                color: library.collection_busy ? root.accent : root.muted
                font.pixelSize: 10
                elide: Text.ElideRight
            }
        }
        footer: Rectangle {
            width: parent.width
            height: 64
            color: root.panelRaised
            radius: 14
            Row {
                anchors.left: parent.left
                anchors.leftMargin: 18
                anchors.verticalCenter: parent.verticalCenter
                spacing: 9
                Button {
                    text: "+ New"
                    enabled: !library.collection_busy
                    onClicked: root.openNewCollectionDialog()
                }
                Button {
                    text: "Edit"
                    enabled: managedCollectionList.currentIndex >= 0
                             && !library.collection_busy
                    onClicked: root.openEditCollectionDialog(
                                   managedCollectionList.currentIndex)
                }
                Button {
                    text: "Delete"
                    enabled: managedCollectionList.currentIndex >= 0
                             && !library.collection_busy
                    onClicked: {
                        root.deletingCollectionId = library.collection_id_at(
                                                    managedCollectionList.currentIndex)
                        root.deletingCollectionName = library.collection_name_at(
                                                      managedCollectionList.currentIndex)
                        deleteCollectionDialog.open()
                    }
                }
            }
        }
    }

    Dialog {
        id: deleteCollectionDialog
        modal: true
        anchors.centerIn: parent
        width: Math.min(460, root.width - 48)
        title: "Delete collection?"
        standardButtons: Dialog.Cancel | Dialog.Ok
        closePolicy: Popup.CloseOnEscape
        onAccepted: {
            library.delete_collection(root.deletingCollectionId)
            root.deletingCollectionId = ""
            root.deletingCollectionName = ""
        }
        onRejected: {
            root.deletingCollectionId = ""
            root.deletingCollectionName = ""
        }
        contentItem: Text {
            text: "Delete ‘" + root.deletingCollectionName
                  + "’? The games and ROM files will not be removed."
            color: root.ink
            font.pixelSize: 13
            wrapMode: Text.WordWrap
        }
    }

    Dialog {
        id: downloadHistoryDialog
        modal: true
        anchors.centerIn: parent
        width: Math.min(470, root.width - 48)
        title: root.clearAllDownloadHistory
               ? "Clear finished download history?"
               : "Remove download record?"
        standardButtons: Dialog.Cancel | Dialog.Ok
        closePolicy: Popup.CloseOnEscape
        onAccepted: {
            if (root.clearAllDownloadHistory)
                downloadQueue.clear_finished()
            else
                downloadQueue.remove_job(root.pendingHistoryJobId)
            root.pendingHistoryJobId = ""
            root.pendingHistoryJobTitle = ""
            root.clearAllDownloadHistory = false
        }
        onRejected: {
            root.pendingHistoryJobId = ""
            root.pendingHistoryJobTitle = ""
            root.clearAllDownloadHistory = false
        }
        contentItem: Text {
            text: root.clearAllDownloadHistory
                  ? "Remove all completed, imported, cancelled, and failed records from Lunchbox history? Active downloads, pending imports, torrents, and downloaded files will be preserved."
                  : "Remove ‘" + root.pendingHistoryJobTitle
                    + "’ from Lunchbox history? Its torrent and downloaded files will be preserved."
            color: root.ink
            font.pixelSize: 13
            wrapMode: Text.WordWrap
        }
    }

    FolderDialog {
        id: directoryDialog
        title: root.directoryTarget === "rom" ? "Choose ROM library" : "Choose torrent library"
        onAccepted: appSettings.set_directory(root.directoryTarget, selectedFolder)
    }

    FolderDialog {
        id: importDirectoryDialog
        title: "Choose a ROM collection to scan"
        onAccepted: localImport.choose_directory(selectedFolder)
    }

    FileDialog {
        id: firmwarePackageDialog
        title: "Choose the exact firmware package shown in Game Details"
        fileMode: FileDialog.OpenFile
        nameFilters: ["Firmware packages (*.zip *.xml *.bin *.rom *.dat)", "All files (*)"]
        onAccepted: gameDetails.import_firmware_package(selectedFile)
    }

    Loader {
        id: importDialogLoader
        anchors.fill: parent
        active: false
        sourceComponent: importDialogComponent
    }

    Component {
        id: importDialogComponent
        Dialog {
            id: importDialog
            modal: true
            anchors.centerIn: parent
            width: Math.min(1060, root.width - 50)
            height: Math.min(790, root.height - 42)
            padding: 0
            closePolicy: Popup.CloseOnEscape

            Timer {
                id: importFilterDelay
                interval: 75
                repeat: false
                onTriggered: localImport.apply_result_filter(
                                 importSearch.text,
                                 importStatus.currentValue,
                                 importSort.currentValue,
                                 importSortAscending.checked)
            }

            Connections {
                target: localImport
                function onResult_countChanged() {
                    if (localImport.result_count > 0)
                        importFilterDelay.restart()
                }
            }

            background: Rectangle {
            color: root.panel
            radius: 14
            border.color: root.line
            border.width: 1
        }

            header: Rectangle {
            width: parent.width
            height: 64
            color: root.panelRaised
            radius: 14
            Column {
                anchors.left: parent.left
                anchors.leftMargin: 22
                anchors.verticalCenter: parent.verticalCenter
                spacing: 2
                Text {
                    text: "IMPORT LOCAL ROMS"
                    color: root.ink
                    font.pixelSize: 17
                    font.weight: Font.Bold
                    font.letterSpacing: 0.8
                }
                Text {
                    text: "Review exact identities before adding files to My Collection"
                    color: root.muted
                    font.pixelSize: 10
                }
            }
            RoundButton {
                anchors.right: parent.right
                anchors.rightMargin: 14
                anchors.verticalCenter: parent.verticalCenter
                text: "×"
                flat: true
                font.pixelSize: 20
                onClicked: importDialog.close()
            }
        }

            contentItem: Item {
            ColumnLayout {
                anchors.fill: parent
                anchors.margins: 20
                spacing: 10

                RowLayout {
                Layout.fillWidth: true
                spacing: 9
                TextField {
                    Layout.fillWidth: true
                    text: localImport.directory
                    readOnly: true
                    placeholderText: "Choose a folder containing ROMs"
                }
                Button {
                    text: "Choose…"
                    enabled: !localImport.busy
                    onClicked: importDirectoryDialog.open()
                }
                ComboBox {
                    id: importPlatform
                    Layout.preferredWidth: 245
                    model: {
                        const values = ["Auto-detect platform"]
                        const revision = localImport.platform_count
                        for (let index = 0; index < revision; ++index)
                            values.push(localImport.platform_name_at(index))
                        return values
                    }
                }
                CheckBox {
                    id: importChecksums
                    text: "Exact checksums"
                    checked: true
                    enabled: !localImport.busy
                    ToolTip.visible: hovered
                    ToolTip.text: checked
                                      ? "Use SHA-1 and MD5 to prove catalog identity"
                                      : "Inventory files without assigning catalog identity"
                }
                HeaderButton {
                    text: localImport.scanning ? "Scanning…" : "Scan"
                    active: true
                    enabled: !localImport.busy && localImport.directory.length > 0
                    onClicked: localImport.start_scan(
                                   importPlatform.currentIndex === 0
                                       ? ""
                                       : localImport.platform_name_at(importPlatform.currentIndex - 1),
                                   importChecksums.checked)
                }
                Button {
                    text: "Cancel"
                    visible: localImport.scanning
                    onClicked: localImport.cancel_scan()
                }
                }

                Rectangle {
                Layout.fillWidth: true
                Layout.preferredHeight: 58
                radius: 9
                color: "#111824"
                border.color: root.line
                RowLayout {
                    anchors.fill: parent
                    anchors.margins: 11
                    BusyIndicator {
                        visible: localImport.busy
                        running: visible
                        Layout.preferredWidth: 22
                        Layout.preferredHeight: 22
                    }
                    ColumnLayout {
                        Layout.fillWidth: true
                        spacing: 4
                        Text {
                            Layout.fillWidth: true
                            text: localImport.message
                            color: root.muted
                            font.pixelSize: 11
                            elide: Text.ElideRight
                        }
                        Rectangle {
                            Layout.fillWidth: true
                            Layout.preferredHeight: 6
                            visible: localImport.scanning
                            radius: 3
                            color: root.line
                            clip: true
                            Rectangle {
                                id: importProgressFill
                                height: parent.height
                                radius: 3
                                color: root.accentCool
                                width: localImport.total_files > 0
                                       ? parent.width * Math.min(1, localImport.scanned_files / localImport.total_files)
                                       : parent.width * 0.28
                                x: localImport.total_files > 0 ? 0 : -width
                                NumberAnimation on x {
                                    running: localImport.scanning && localImport.total_files === 0
                                    from: -importProgressFill.width
                                    to: importProgressFill.parent.width
                                    duration: 850
                                    loops: Animation.Infinite
                                }
                            }
                        }
                    }
                    Text {
                        visible: localImport.total_files > 0
                        text: localImport.matched_count + " exact  ·  "
                              + localImport.unmatched_count + " other"
                        color: root.accentCool
                        font.pixelSize: 11
                        font.weight: Font.DemiBold
                    }
                }
                }

                RowLayout {
                Layout.fillWidth: true
                visible: localImport.total_files > 0
                spacing: 8
                TextField {
                    id: importSearch
                    Layout.fillWidth: true
                    placeholderText: "Filter scan results"
                    onTextEdited: importFilterDelay.restart()
                }
                ComboBox {
                    id: importStatus
                    Layout.preferredWidth: 150
                    textRole: "label"
                    valueRole: "value"
                    model: [
                        { label: "All results", value: "all" },
                        { label: "Exact", value: "exact" },
                        { label: "Ambiguous", value: "ambiguous" },
                        { label: "Unmatched", value: "unmatched" },
                        { label: "Inventory only", value: "inventory_only" },
                        { label: "Errors", value: "error" }
                    ]
                    onActivated: importFilterDelay.restart()
                }
                ComboBox {
                    id: importSort
                    Layout.preferredWidth: 135
                    textRole: "label"
                    valueRole: "value"
                    model: [
                        { label: "File name", value: "name" },
                        { label: "Match status", value: "status" },
                        { label: "Platform", value: "platform" },
                        { label: "File size", value: "size" }
                    ]
                    currentIndex: 1
                    onActivated: importFilterDelay.restart()
                }
                ToolButton {
                    id: importSortAscending
                    checkable: true
                    checked: true
                    text: checked ? "↑" : "↓"
                    onClicked: importFilterDelay.restart()
                    ToolTip.visible: hovered
                    ToolTip.text: checked ? "Ascending" : "Descending"
                }
                Button {
                    text: "Exact"
                    onClicked: localImport.select_visible("exact")
                    ToolTip.visible: hovered
                    ToolTip.text: "Select only proven matches in the current results"
                }
                Button {
                    text: "All"
                    onClicked: localImport.select_visible("all")
                }
                Button {
                    text: "None"
                    onClicked: localImport.select_visible("none")
                }
                }

                Rectangle {
                Layout.fillWidth: true
                Layout.preferredHeight: 34
                visible: localImport.total_files > 0
                color: root.panelRaised
                border.color: root.line
                Row {
                    anchors.fill: parent
                    anchors.leftMargin: 45
                    spacing: 0
                    Text { width: 250; anchors.verticalCenter: parent.verticalCenter; text: "ROM FILE"; color: root.muted; font.pixelSize: 9; font.weight: Font.Bold }
                    Text { width: 155; anchors.verticalCenter: parent.verticalCenter; text: "STATUS"; color: root.muted; font.pixelSize: 9; font.weight: Font.Bold }
                    Text { width: 190; anchors.verticalCenter: parent.verticalCenter; text: "PLATFORM"; color: root.muted; font.pixelSize: 9; font.weight: Font.Bold }
                    Text { anchors.verticalCenter: parent.verticalCenter; text: "MATCH / DETAILS"; color: root.muted; font.pixelSize: 9; font.weight: Font.Bold }
                }
                }

                ListView {
                id: importResults
                Layout.fillWidth: true
                Layout.fillHeight: true
                clip: true
                reuseItems: true
                spacing: 3
                model: localImport.result_count
                ScrollBar.vertical: ScrollBar { policy: ScrollBar.AsNeeded }
                delegate: Rectangle {
                    id: importRow
                    required property int index
                    property int importRevision: localImport.revision
                    width: importResults.width
                    height: 61
                    radius: 7
                    color: rowHover.hovered ? "#202a38" : index % 2 ? "#141b26" : "#111824"
                    border.color: root.line
                    HoverHandler { id: rowHover }
                    ToolTip.visible: rowHover.hovered
                    ToolTip.text: { importRevision; return localImport.result_path_at(index) }
                    CheckBox {
                        anchors.left: parent.left
                        anchors.leftMargin: 9
                        anchors.verticalCenter: parent.verticalCenter
                        checked: { importRow.importRevision; return localImport.result_selected_at(importRow.index) }
                        onClicked: localImport.toggle_selected(importRow.index)
                    }
                    Column {
                        anchors.left: parent.left
                        anchors.leftMargin: 45
                        anchors.verticalCenter: parent.verticalCenter
                        width: 240
                        spacing: 3
                        Text {
                            width: parent.width
                            text: { importRow.importRevision; return localImport.result_file_name_at(importRow.index) }
                            color: root.ink
                            font.pixelSize: 11
                            elide: Text.ElideMiddle
                        }
                        Text {
                            width: parent.width
                            visible: text.length > 0
                            text: { importRow.importRevision; return localImport.result_archive_detail_at(importRow.index) }
                            color: root.accentCool
                            font.pixelSize: 9
                            elide: Text.ElideMiddle
                        }
                    }
                    Rectangle {
                        anchors.left: parent.left
                        anchors.leftMargin: 295
                        anchors.verticalCenter: parent.verticalCenter
                        width: 125
                        height: 24
                        radius: 12
                        color: statusText.text === "EXACT" ? "#18302c"
                             : statusText.text === "ERROR" ? "#351d24" : "#292631"
                        border.color: statusText.text === "EXACT" ? root.accentCool
                                    : statusText.text === "ERROR" ? "#ef7182" : root.accent
                        Text {
                            id: statusText
                            anchors.centerIn: parent
                            text: { importRow.importRevision; return localImport.result_status_at(importRow.index) }
                            color: parent.border.color
                            font.pixelSize: 8
                            font.weight: Font.Bold
                            font.letterSpacing: 0.7
                        }
                    }
                    Text {
                        anchors.left: parent.left
                        anchors.leftMargin: 430
                        anchors.verticalCenter: parent.verticalCenter
                        width: 180
                        text: { importRow.importRevision; return localImport.result_platform_at(importRow.index) }
                        color: root.muted
                        font.pixelSize: 10
                        elide: Text.ElideRight
                    }
                    Column {
                        anchors.left: parent.left
                        anchors.leftMargin: 620
                        anchors.right: sizeLabel.left
                        anchors.rightMargin: 10
                        anchors.verticalCenter: parent.verticalCenter
                        spacing: 3
                        Text {
                            width: parent.width
                            text: { importRow.importRevision; return localImport.result_title_at(importRow.index) }
                            color: root.ink
                            font.pixelSize: 11
                            elide: Text.ElideRight
                        }
                        Text {
                            width: parent.width
                            text: { importRow.importRevision; return localImport.result_detail_at(importRow.index) }
                            color: root.muted
                            font.pixelSize: 9
                            elide: Text.ElideRight
                        }
                    }
                    Text {
                        id: sizeLabel
                        anchors.right: parent.right
                        anchors.rightMargin: 12
                        anchors.verticalCenter: parent.verticalCenter
                        width: 72
                        horizontalAlignment: Text.AlignRight
                        text: { importRow.importRevision; return localImport.result_size_at(importRow.index) }
                        color: root.muted
                        font.pixelSize: 9
                    }
                }
                }

                RowLayout {
                Layout.fillWidth: true
                Text {
                    Layout.fillWidth: true
                    text: localImport.selected_count
                          + " selected · multi-member archives require explicit review; unmatched files stay local-only"
                    color: root.muted
                    font.pixelSize: 11
                }
                Button {
                    text: "Close"
                    onClicked: importDialog.close()
                }
                HeaderButton {
                    text: localImport.importing ? "Importing…" : "Import " + localImport.selected_count
                    active: true
                    enabled: !localImport.busy && localImport.selected_count > 0
                    onClicked: localImport.import_selected()
                }
                }
            }
            }
        }
    }

    Dialog {
        id: settingsDialog
        modal: true
        anchors.centerIn: parent
        width: Math.min(root.controllerProfileUiProbe ? 1120 : 760,
                        root.width - 60)
        height: Math.min(root.controllerProfileUiProbe ? 1080 : 760,
                         root.height - 60)
        padding: 0
        closePolicy: Popup.CloseOnEscape
        onOpened: appSettings.refresh_controllers()

        background: Rectangle {
            color: root.panel
            radius: 14
            border.color: root.line
            border.width: 1
        }

        header: Rectangle {
            width: parent.width
            height: 62
            color: root.panelRaised
            radius: 14
            Text {
                anchors.left: parent.left
                anchors.leftMargin: 22
                anchors.verticalCenter: parent.verticalCenter
                text: "SETTINGS"
                color: root.ink
                font.pixelSize: 17
                font.weight: Font.Bold
                font.letterSpacing: 0.8
            }
            RoundButton {
                anchors.right: parent.right
                anchors.rightMargin: 14
                anchors.verticalCenter: parent.verticalCenter
                text: "×"
                flat: true
                font.pixelSize: 20
                onClicked: settingsDialog.close()
            }
        }

        contentItem: ScrollView {
            id: settingsScroll
            clip: true
            ScrollBar.horizontal.policy: ScrollBar.AlwaysOff
            ColumnLayout {
                x: 24
                width: settingsDialog.availableWidth - 48
                spacing: 11

                Text {
                    text: "QBITTORRENT"
                    color: root.accent
                    font.pixelSize: 10
                    font.weight: Font.Bold
                    font.letterSpacing: 1.2
                }
                Text {
                    Layout.fillWidth: true
                    text: "Lunchbox uses the Web API and an isolated ‘lunchbox’ category. Credentials are stored in the operating system credential store, never in SQLite."
                    color: root.muted
                    font.pixelSize: 11
                    wrapMode: Text.WordWrap
                }
                RowLayout {
                    Layout.fillWidth: true
                    spacing: 9
                    TextField {
                        Layout.fillWidth: true
                        placeholderText: "Host"
                        text: appSettings.qbittorrent_host
                        onTextEdited: appSettings.qbittorrent_host = text
                    }
                    SpinBox {
                        Layout.preferredWidth: 125
                        from: 1
                        to: 65535
                        editable: true
                        value: appSettings.qbittorrent_port
                        onValueModified: appSettings.qbittorrent_port = value
                    }
                    Switch {
                        text: "HTTPS"
                        checked: appSettings.qbittorrent_use_https
                        onToggled: appSettings.qbittorrent_use_https = checked
                    }
                }
                RowLayout {
                    Layout.fillWidth: true
                    spacing: 9
                    TextField {
                        Layout.fillWidth: true
                        placeholderText: "Username"
                        text: appSettings.qbittorrent_username
                        onTextEdited: appSettings.qbittorrent_username = text
                    }
                    TextField {
                        Layout.fillWidth: true
                        placeholderText: appSettings.password_saved ? "Saved password (leave blank to keep)" : "Password"
                        echoMode: TextInput.Password
                        text: appSettings.qbittorrent_password
                        onTextEdited: appSettings.qbittorrent_password = text
                    }
                    Button {
                        visible: appSettings.password_saved
                        text: "Clear saved"
                        enabled: !appSettings.busy
                        onClicked: appSettings.clear_password()
                    }
                }

                Rectangle { Layout.fillWidth: true; Layout.preferredHeight: 1; color: root.line }
                Text {
                    text: "LIBRARY PATHS"
                    color: root.accent
                    font.pixelSize: 10
                    font.weight: Font.Bold
                    font.letterSpacing: 1.2
                }
                Text {
                    Layout.fillWidth: true
                    text: "Native paths are interpreted by this operating system. Client paths are passed verbatim to qBittorrent, which supports containers and remote clients without Linux-specific path assumptions."
                    color: root.muted
                    font.pixelSize: 11
                    wrapMode: Text.WordWrap
                }
                RowLayout {
                    Layout.fillWidth: true
                    TextField {
                        Layout.fillWidth: true
                        placeholderText: "Native ROM library"
                        text: appSettings.rom_directory
                        readOnly: true
                    }
                    Button {
                        text: "Choose…"
                        onClicked: {
                            root.directoryTarget = "rom"
                            directoryDialog.open()
                        }
                    }
                }
                TextField {
                    Layout.fillWidth: true
                    placeholderText: "qBittorrent/container ROM path (for emulator workflows)"
                    text: appSettings.qbittorrent_container_rom_directory
                    onTextEdited: appSettings.qbittorrent_container_rom_directory = text
                }
                RowLayout {
                    Layout.fillWidth: true
                    TextField {
                        Layout.fillWidth: true
                        placeholderText: "Native torrent library"
                        text: appSettings.torrent_library_directory
                        readOnly: true
                    }
                    Button {
                        text: "Choose…"
                        onClicked: {
                            root.directoryTarget = "torrent"
                            directoryDialog.open()
                        }
                    }
                }
                TextField {
                    Layout.fillWidth: true
                    placeholderText: "qBittorrent/container torrent library path"
                    text: appSettings.qbittorrent_container_torrent_library_directory
                    onTextEdited: appSettings.qbittorrent_container_torrent_library_directory = text
                }
                RowLayout {
                    Layout.fillWidth: true
                    Text { text: "Install completed files using"; color: root.ink; font.pixelSize: 12 }
                    ComboBox {
                        id: linkMode
                        Layout.fillWidth: true
                        textRole: "label"
                        valueRole: "value"
                        model: [
                            { label: "Symbolic link", value: "symlink" },
                            { label: "Hard link", value: "hardlink" },
                            { label: "Copy-on-write reflink", value: "reflink" },
                            { label: "Full copy", value: "copy" },
                            { label: "Leave in download library", value: "leave_in_place" }
                        ]
                        currentIndex: {
                            for (let i = 0; i < model.length; ++i) {
                                if (model[i].value === appSettings.file_link_mode) {
                                    return i
                                }
                            }
                            return 0
                        }
                        onActivated: appSettings.file_link_mode = currentValue
                    }
                    CheckBox {
                        text: "Entire torrent"
                        checked: appSettings.download_entire_torrent
                        onToggled: appSettings.download_entire_torrent = checked
                    }
                }
                RowLayout {
                    Layout.fillWidth: true
                    spacing: 9
                    Text {
                        text: "After a successful import"
                        color: root.ink
                        font.pixelSize: 12
                    }
                    ComboBox {
                        id: seedingPolicy
                        Layout.fillWidth: true
                        textRole: "label"
                        valueRole: "value"
                        model: [
                            { label: "Follow qBittorrent seeding rules", value: "follow_client" },
                            { label: "Pause the Lunchbox torrent", value: "pause_after_import" }
                        ]
                        currentIndex: appSettings.seeding_policy === "pause_after_import" ? 1 : 0
                        onActivated: appSettings.seeding_policy = currentValue
                    }
                }
                Text {
                    Layout.fillWidth: true
                    text: "Pause is non-destructive. Lunchbox waits for every selected file in a bundled torrent to import, then pauses the torrent without deleting its record or downloaded files."
                    color: root.muted
                    font.pixelSize: 11
                    wrapMode: Text.WordWrap
                }

                Rectangle { Layout.fillWidth: true; Layout.preferredHeight: 1; color: root.line }
                Text {
                    text: "DISCOVERY & DOWNLOADS"
                    color: root.accent
                    font.pixelSize: 10
                    font.weight: Font.Bold
                    font.letterSpacing: 1.2
                }
                Text {
                    Layout.fillWidth: true
                    text: "Order every region used to rank otherwise equal Minerva candidates. Lunchbox still shows the exact region, revision, filename, and size for review before download."
                    color: root.muted
                    font.pixelSize: 11
                    wrapMode: Text.WordWrap
                }
                ColumnLayout {
                    id: regionPrioritySection
                    Layout.fillWidth: true
                    spacing: 7
                    RowLayout {
                        Layout.fillWidth: true
                        Text {
                            Layout.fillWidth: true
                            text: "REGION PRIORITY"
                            color: root.ink
                            font.pixelSize: 12
                            font.weight: Font.DemiBold
                        }
                        Button {
                            text: "Reset defaults"
                            enabled: !appSettings.busy
                            onClicked: appSettings.reset_region_priority()
                        }
                    }
                    Rectangle {
                        Layout.fillWidth: true
                        Layout.preferredHeight: 258
                        radius: 9
                        color: "#0f151f"
                        border.color: root.line
                        border.width: 1

                        ListView {
                            id: regionPriorityList
                            anchors.fill: parent
                            anchors.margins: 7
                            clip: true
                            spacing: 4
                            boundsBehavior: Flickable.StopAtBounds
                            model: {
                                appSettings.region_revision
                                return appSettings.region_count()
                            }
                            ScrollBar.vertical: ScrollBar { }

                            delegate: Rectangle {
                                id: regionRow
                                required property int index
                                width: regionPriorityList.width - 10
                                height: 36
                                radius: 7
                                color: index < 3 ? "#1c2a35" : "#161e29"
                                border.color: index === 0 ? root.accent : "transparent"
                                border.width: 1

                                RowLayout {
                                    anchors.fill: parent
                                    anchors.leftMargin: 10
                                    anchors.rightMargin: 5
                                    spacing: 8
                                    Text {
                                        Layout.preferredWidth: 22
                                        text: (regionRow.index + 1).toString().padStart(2, "0")
                                        color: regionRow.index === 0 ? root.accent : root.muted
                                        font.pixelSize: 10
                                        font.family: "monospace"
                                    }
                                    Text {
                                        Layout.fillWidth: true
                                        text: {
                                            appSettings.region_revision
                                            return appSettings.region_at(regionRow.index)
                                        }
                                        color: root.ink
                                        font.pixelSize: 12
                                        font.weight: regionRow.index === 0
                                                     ? Font.DemiBold : Font.Normal
                                    }
                                    ToolButton {
                                        text: "↑"
                                        enabled: regionRow.index > 0 && !appSettings.busy
                                        Accessible.name: "Move region up"
                                        onClicked: appSettings.move_region(
                                                       regionRow.index,
                                                       regionRow.index - 1)
                                    }
                                    ToolButton {
                                        text: "↓"
                                        enabled: regionRow.index + 1
                                                 < appSettings.region_count()
                                                 && !appSettings.busy
                                        Accessible.name: "Move region down"
                                        onClicked: appSettings.move_region(
                                                       regionRow.index,
                                                       regionRow.index + 1)
                                    }
                                }
                            }
                        }
                    }
                    Text {
                        Layout.fillWidth: true
                        text: "Use the arrow controls or keyboard focus to reorder. The first matching region wins; untagged releases are ordered explicitly as ‘No region’."
                        color: root.muted
                        font.pixelSize: 10
                        wrapMode: Text.WordWrap
                    }
                }
                RowLayout {
                    Layout.fillWidth: true
                    spacing: 9
                    Text { text: "Release version"; color: root.ink; font.pixelSize: 12 }
                    ComboBox {
                        Layout.fillWidth: true
                        textRole: "label"
                        valueRole: "value"
                        model: [
                            { label: "Latest revision", value: "latest" },
                            { label: "Original / unrevisioned", value: "original" }
                        ]
                        currentIndex: appSettings.version_preference === "original" ? 1 : 0
                        onActivated: appSettings.version_preference = currentValue
                    }
                }

                Rectangle { Layout.fillWidth: true; Layout.preferredHeight: 1; color: root.line }
                ColumnLayout {
                    id: controllerSection
                    Layout.fillWidth: true
                    spacing: 9

                    RowLayout {
                        Layout.fillWidth: true
                        spacing: 12
                        ColumnLayout {
                            Layout.fillWidth: true
                            spacing: 2
                            Text {
                                text: "CONTROLLERS"
                                color: root.accent
                                font.pixelSize: 10
                                font.weight: Font.Bold
                                font.letterSpacing: 1.2
                            }
                            Text {
                                Layout.fillWidth: true
                                text: "Set a stable player order, then remap, pass through, or hide each physical controller while a game is running. Original host state is restored when the emulator exits."
                                color: root.muted
                                font.pixelSize: 11
                                wrapMode: Text.WordWrap
                            }
                        }
                        Switch {
                            text: "Launch mapping"
                            checked: appSettings.controller_enabled
                            enabled: !appSettings.busy
                                     && !appSettings.controller_busy
                            onToggled: appSettings.set_controller_mapping_enabled(checked)
                            Accessible.name: "Enable controller mapping at game launch"
                        }
                    }

                    Rectangle {
                        Layout.fillWidth: true
                        Layout.preferredHeight: controllerStatusText.implicitHeight + 22
                        radius: 8
                        color: "#111a25"
                        border.color: root.line
                        RowLayout {
                            anchors.fill: parent
                            anchors.margins: 10
                            spacing: 9
                            BusyIndicator {
                                visible: appSettings.controller_busy
                                running: visible
                                Layout.preferredWidth: 18
                                Layout.preferredHeight: 18
                            }
                            Text {
                                id: controllerStatusText
                                Layout.fillWidth: true
                                text: appSettings.controller_status
                                color: root.muted
                                font.pixelSize: 10
                                wrapMode: Text.WordWrap
                            }
                            Button {
                                text: "Scan again"
                                enabled: !appSettings.controller_busy
                                onClicked: appSettings.refresh_controllers()
                                Accessible.name: "Scan for connected controllers"
                            }
                        }
                    }

                    RowLayout {
                        Layout.fillWidth: true
                        visible: {
                            appSettings.controller_revision
                            return appSettings.controller_target_count() > 1
                        }
                        spacing: 9
                        Text {
                            text: "Default virtual controller"
                            color: root.ink
                            font.pixelSize: 12
                        }
                        ComboBox {
                            id: defaultControllerTarget
                            Layout.fillWidth: true
                            enabled: appSettings.controller_enabled
                                     && !appSettings.controller_busy
                            model: {
                                appSettings.controller_revision
                                return Math.max(0, appSettings.controller_target_count() - 1)
                            }
                            delegate: ItemDelegate {
                                required property int index
                                width: defaultControllerTarget.width
                                text: appSettings.controller_target_name_at(index + 1)
                            }
                            currentIndex: {
                                appSettings.controller_revision
                                for (let i = 1; i < appSettings.controller_target_count(); ++i) {
                                    if (appSettings.controller_target_id_at(i)
                                            === appSettings.controller_output_target)
                                        return i - 1
                                }
                                return 0
                            }
                            displayText: model > 0
                                         ? appSettings.controller_target_name_at(currentIndex + 1)
                                         : "No virtual targets available"
                            onActivated: appSettings.choose_default_controller_target(
                                             appSettings.controller_target_id_at(currentIndex + 1))
                            Accessible.name: "Default virtual controller target"
                        }
                    }

                    Rectangle {
                        Layout.fillWidth: true
                        Layout.preferredHeight: {
                            appSettings.controller_revision
                            return appSettings.controller_count() === 0
                                                ? 92
                                                : Math.min(440,
                                                           controllerList.contentHeight + 14)
                        }
                        radius: 9
                        color: "#0f151f"
                        border.color: root.line
                        border.width: 1

                        ColumnLayout {
                            anchors.centerIn: parent
                            width: parent.width - 44
                            visible: {
                                appSettings.controller_revision
                                return !appSettings.controller_busy
                                        && appSettings.controller_count() === 0
                            }
                            spacing: 5
                            Text {
                                Layout.alignment: Qt.AlignHCenter
                                text: "NO GAME CONTROLLERS CONNECTED"
                                color: root.ink
                                font.pixelSize: 12
                                font.weight: Font.DemiBold
                            }
                            Text {
                                Layout.fillWidth: true
                                text: "Connect a joystick or gamepad, then scan again. Keyboard, pointer, LED, and non-game virtual devices are intentionally ignored."
                                color: root.muted
                                font.pixelSize: 10
                                horizontalAlignment: Text.AlignHCenter
                                wrapMode: Text.WordWrap
                            }
                        }

                        ListView {
                            id: controllerList
                            anchors.fill: parent
                            anchors.margins: 7
                            visible: {
                                appSettings.controller_revision
                                return appSettings.controller_count() > 0
                            }
                            clip: true
                            spacing: 6
                            boundsBehavior: Flickable.StopAtBounds
                            model: {
                                appSettings.controller_revision
                                return appSettings.controller_count()
                            }
                            ScrollBar.vertical: ScrollBar { }

                            delegate: Rectangle {
                                id: controllerRow
                                required property int index
                                width: controllerList.width - 10
                                height: 142
                                radius: 8
                                color: index === 0 ? "#1c2a35" : "#161e29"
                                border.color: index === 0 ? root.accent : "transparent"
                                border.width: 1

                                ColumnLayout {
                                    anchors.fill: parent
                                    anchors.margins: 10
                                    spacing: 6
                                    RowLayout {
                                        Layout.fillWidth: true
                                        spacing: 8
                                        Rectangle {
                                            Layout.preferredWidth: 34
                                            Layout.preferredHeight: 26
                                            radius: 6
                                            color: root.accent
                                            Text {
                                                anchors.centerIn: parent
                                                text: "P" + (controllerRow.index + 1)
                                                color: "#071116"
                                                font.pixelSize: 11
                                                font.weight: Font.Bold
                                            }
                                        }
                                        ColumnLayout {
                                            Layout.fillWidth: true
                                            spacing: 1
                                            Text {
                                                Layout.fillWidth: true
                                                text: appSettings.controller_name_at(controllerRow.index)
                                                color: root.ink
                                                font.pixelSize: 12
                                                font.weight: Font.DemiBold
                                                elide: Text.ElideRight
                                            }
                                            Text {
                                                Layout.fillWidth: true
                                                text: appSettings.controller_detail_at(controllerRow.index)
                                                color: root.muted
                                                font.pixelSize: 9
                                                elide: Text.ElideMiddle
                                            }
                                        }
                                        ToolButton {
                                            text: "↑"
                                            enabled: controllerRow.index > 0
                                                     && !appSettings.controller_busy
                                            onClicked: appSettings.move_controller(
                                                           controllerRow.index, -1)
                                            Accessible.name: "Move controller up in player order"
                                        }
                                        ToolButton {
                                            text: "↓"
                                            enabled: controllerRow.index + 1
                                                     < appSettings.controller_count()
                                                     && !appSettings.controller_busy
                                            onClicked: appSettings.move_controller(
                                                           controllerRow.index, 1)
                                            Accessible.name: "Move controller down in player order"
                                        }
                                    }

                                    RowLayout {
                                        Layout.fillWidth: true
                                        spacing: 7
                                        ComboBox {
                                            id: controllerAction
                                            Layout.preferredWidth: 180
                                            textRole: "label"
                                            valueRole: "value"
                                            model: [
                                                { label: "Remap", value: "remap" },
                                                { label: "Pass through", value: "passthrough" },
                                                { label: "Hide from emulator", value: "hide" }
                                            ]
                                            currentIndex: {
                                                appSettings.controller_revision
                                                const value = appSettings.controller_action_at(
                                                                  controllerRow.index)
                                                for (let i = 0; i < model.length; ++i) {
                                                    if (model[i].value === value)
                                                        return i
                                                }
                                                return 0
                                            }
                                            enabled: appSettings.controller_enabled
                                                     && !appSettings.controller_busy
                                            onActivated: appSettings.choose_controller_action(
                                                             controllerRow.index, currentValue)
                                            Accessible.name: "Controller action for player "
                                                             + (controllerRow.index + 1)
                                        }
                                        ComboBox {
                                            id: controllerProfile
                                            Layout.fillWidth: true
                                            model: {
                                                appSettings.controller_revision
                                                return appSettings.controller_profile_count()
                                            }
                                            delegate: ItemDelegate {
                                                required property int index
                                                width: controllerProfile.width
                                                text: appSettings.controller_profile_name_at(index)
                                            }
                                            currentIndex: {
                                                appSettings.controller_revision
                                                const value = appSettings.controller_profile_at(
                                                                  controllerRow.index)
                                                for (let i = 0;
                                                     i < appSettings.controller_profile_count();
                                                     ++i) {
                                                    if (appSettings.controller_profile_id_at(i)
                                                        === value)
                                                        return i
                                                }
                                                return 0
                                            }
                                            displayText: appSettings.controller_profile_name_at(
                                                             currentIndex)
                                            enabled: appSettings.controller_enabled
                                                     && controllerAction.currentValue === "remap"
                                                     && !appSettings.controller_busy
                                            onActivated: appSettings.choose_controller_profile(
                                                             controllerRow.index,
                                                             appSettings.controller_profile_id_at(
                                                                 currentIndex))
                                            Accessible.name: "Controller profile for player "
                                                             + (controllerRow.index + 1)
                                        }
                                        ComboBox {
                                            id: controllerTarget
                                            Layout.fillWidth: true
                                            model: {
                                                appSettings.controller_revision
                                                return appSettings.controller_target_count()
                                            }
                                            delegate: ItemDelegate {
                                                required property int index
                                                width: controllerTarget.width
                                                text: appSettings.controller_target_name_at(index)
                                            }
                                            currentIndex: {
                                                appSettings.controller_revision
                                                const value = appSettings.controller_target_at(
                                                                  controllerRow.index)
                                                for (let i = 0;
                                                     i < appSettings.controller_target_count();
                                                     ++i) {
                                                    if (appSettings.controller_target_id_at(i)
                                                        === value)
                                                        return i
                                                }
                                                return 0
                                            }
                                            displayText: appSettings.controller_target_name_at(
                                                             currentIndex)
                                            enabled: appSettings.controller_enabled
                                                     && controllerAction.currentValue === "remap"
                                                     && !appSettings.controller_busy
                                            onActivated: appSettings.choose_controller_target(
                                                             controllerRow.index,
                                                             appSettings.controller_target_id_at(
                                                                 currentIndex))
                                            Accessible.name: "Virtual controller target for player "
                                                             + (controllerRow.index + 1)
                                        }
                                    }
                                }
                            }
                        }
                    }

                    Rectangle {
                        Layout.fillWidth: true
                        Layout.preferredHeight: 1
                        color: root.line
                    }

                    RowLayout {
                        Layout.fillWidth: true
                        spacing: 10
                        ColumnLayout {
                            Layout.fillWidth: true
                            spacing: 2
                            Text {
                                text: "CUSTOM PROFILES"
                                color: root.accent
                                font.pixelSize: 10
                                font.weight: Font.Bold
                                font.letterSpacing: 1.1
                            }
                            Text {
                                Layout.fillWidth: true
                                text: "Build reusable physical-to-virtual button maps. Identity mappings stay implicit, so profiles contain only deliberate changes."
                                color: root.muted
                                font.pixelSize: 10
                                wrapMode: Text.WordWrap
                            }
                        }
                        Button {
                            text: "New profile"
                            enabled: !appSettings.controller_profile_editor_open
                            onClicked: appSettings.create_controller_profile()
                            Accessible.name: "Create a custom controller profile"
                        }
                    }

                    Text {
                        Layout.fillWidth: true
                        visible: !appSettings.controller_profile_editor_open
                                 && appSettings.controller_profile_editor_status.length > 0
                        text: appSettings.controller_profile_editor_status
                        color: root.accentCool
                        font.pixelSize: 10
                        wrapMode: Text.WordWrap
                    }

                    Rectangle {
                        Layout.fillWidth: true
                        Layout.preferredHeight: {
                            appSettings.controller_profile_revision
                            const count = appSettings.custom_controller_profile_count()
                            return !visible ? 0 : count === 0
                                                 ? 74
                                                 : Math.min(270, count * 64 + 12)
                        }
                        visible: !appSettings.controller_profile_editor_open
                        radius: 9
                        color: "#0f151f"
                        border.color: root.line

                        ColumnLayout {
                            anchors.centerIn: parent
                            width: parent.width - 36
                            visible: {
                                appSettings.controller_profile_revision
                                return appSettings.custom_controller_profile_count() === 0
                            }
                            spacing: 3
                            Text {
                                Layout.alignment: Qt.AlignHCenter
                                text: "NO CUSTOM PROFILES"
                                color: root.ink
                                font.pixelSize: 11
                                font.weight: Font.DemiBold
                            }
                            Text {
                                Layout.fillWidth: true
                                text: "Create one here, then assign it to any connected controller from the Profile menu."
                                color: root.muted
                                font.pixelSize: 9
                                horizontalAlignment: Text.AlignHCenter
                                wrapMode: Text.WordWrap
                            }
                        }

                        ListView {
                            id: customControllerProfileList
                            anchors.fill: parent
                            anchors.margins: 6
                            visible: {
                                appSettings.controller_profile_revision
                                return appSettings.custom_controller_profile_count() > 0
                            }
                            clip: true
                            spacing: 5
                            boundsBehavior: Flickable.StopAtBounds
                            model: {
                                appSettings.controller_profile_revision
                                return appSettings.custom_controller_profile_count()
                            }
                            ScrollBar.vertical: ScrollBar { }

                            delegate: Rectangle {
                                id: customProfileRow
                                required property int index
                                width: customControllerProfileList.width - 8
                                height: 58
                                radius: 7
                                color: "#161e29"
                                border.color: "#202b3b"

                                RowLayout {
                                    anchors.fill: parent
                                    anchors.margins: 9
                                    spacing: 8
                                    Rectangle {
                                        Layout.preferredWidth: 30
                                        Layout.preferredHeight: 30
                                        radius: 8
                                        color: "#202d3b"
                                        Text {
                                            anchors.centerIn: parent
                                            text: "⌁"
                                            color: root.accentCool
                                            font.pixelSize: 16
                                            font.weight: Font.Bold
                                        }
                                    }
                                    ColumnLayout {
                                        Layout.fillWidth: true
                                        spacing: 1
                                        Text {
                                            Layout.fillWidth: true
                                            text: appSettings.custom_controller_profile_name_at(
                                                      customProfileRow.index)
                                            color: root.ink
                                            font.pixelSize: 11
                                            font.weight: Font.DemiBold
                                            elide: Text.ElideRight
                                        }
                                        Text {
                                            Layout.fillWidth: true
                                            text: appSettings.custom_controller_profile_detail_at(
                                                      customProfileRow.index)
                                            color: root.muted
                                            font.pixelSize: 9
                                            elide: Text.ElideRight
                                        }
                                    }
                                    Button {
                                        text: "Edit"
                                        onClicked: appSettings.edit_controller_profile(
                                                       customProfileRow.index)
                                        Accessible.name: "Edit custom controller profile"
                                    }
                                    Button {
                                        text: "Delete"
                                        onClicked: {
                                            root.pendingControllerProfileDeleteIndex =
                                                customProfileRow.index
                                            root.pendingControllerProfileDeleteName =
                                                appSettings.custom_controller_profile_name_at(
                                                    customProfileRow.index)
                                            controllerProfileDeleteDialog.open()
                                        }
                                        Accessible.name: "Delete custom controller profile"
                                    }
                                }
                            }
                        }
                    }

                    Rectangle {
                        id: controllerProfileEditor
                        Layout.fillWidth: true
                        Layout.preferredHeight: visible ? 650 : 0
                        visible: appSettings.controller_profile_editor_open
                        radius: 10
                        color: "#101925"
                        border.color: root.accentCool
                        border.width: 1

                        ColumnLayout {
                            anchors.fill: parent
                            anchors.margins: 14
                            spacing: 11

                            RowLayout {
                                Layout.fillWidth: true
                                spacing: 10
                                ColumnLayout {
                                    Layout.fillWidth: true
                                    spacing: 2
                                    Text {
                                        text: appSettings.controller_profile_editor_id.length > 0
                                              ? "EDIT CONTROLLER PROFILE"
                                              : "CREATE CONTROLLER PROFILE"
                                        color: root.ink
                                        font.pixelSize: 14
                                        font.weight: Font.Bold
                                        font.letterSpacing: 0.6
                                    }
                                    Text {
                                        text: "Select a virtual button, then choose which physical button should produce it."
                                        color: root.muted
                                        font.pixelSize: 9
                                    }
                                }
                                Button {
                                    text: "Cancel"
                                    onClicked: appSettings.cancel_controller_profile_edit()
                                    Accessible.name: "Cancel controller profile editing"
                                }
                            }

                            RowLayout {
                                Layout.fillWidth: true
                                spacing: 10
                                ColumnLayout {
                                    Layout.fillWidth: true
                                    spacing: 4
                                    Text {
                                        text: "Profile name"
                                        color: root.muted
                                        font.pixelSize: 9
                                    }
                                    TextField {
                                        id: controllerProfileName
                                        Layout.fillWidth: true
                                        text: appSettings.controller_profile_editor_name
                                        maximumLength: 100
                                        selectByMouse: true
                                        onTextEdited: appSettings.controller_profile_editor_name = text
                                        Accessible.name: "Custom controller profile name"
                                    }
                                }
                                ColumnLayout {
                                    Layout.preferredWidth: 210
                                    spacing: 4
                                    Text {
                                        text: "Button labels"
                                        color: root.muted
                                        font.pixelSize: 9
                                    }
                                    ComboBox {
                                        Layout.fillWidth: true
                                        textRole: "label"
                                        valueRole: "value"
                                        model: [
                                            { label: "Xbox", value: "xbox" },
                                            { label: "PlayStation 4 / 5", value: "playstation" },
                                            { label: "Generic", value: "generic" }
                                        ]
                                        currentIndex: appSettings.controller_profile_editor_layout
                                                      === "playstation" ? 1
                                                      : appSettings.controller_profile_editor_layout
                                                        === "generic" ? 2 : 0
                                        onActivated: appSettings.controller_profile_editor_layout =
                                                         currentValue
                                        Accessible.name: "Controller diagram label style"
                                    }
                                }
                            }

                            RowLayout {
                                Layout.fillWidth: true
                                Layout.preferredHeight: 330
                                spacing: 11

                                Rectangle {
                                    id: controllerDiagram
                                    Layout.fillWidth: true
                                    Layout.fillHeight: true
                                    radius: 9
                                    color: "#0b121b"
                                    border.color: root.line
                                    readonly property var buttons: [
                                        { id: "LeftTrigger", x: 88, y: 30 },
                                        { id: "LeftBumper", x: 88, y: 62 },
                                        { id: "RightTrigger", x: 292, y: 30 },
                                        { id: "RightBumper", x: 292, y: 62 },
                                        { id: "DPadUp", x: 88, y: 128 },
                                        { id: "DPadLeft", x: 62, y: 154 },
                                        { id: "DPadRight", x: 114, y: 154 },
                                        { id: "DPadDown", x: 88, y: 180 },
                                        { id: "LeftStick", x: 142, y: 188 },
                                        { id: "Select", x: 168, y: 126 },
                                        { id: "Guide", x: 190, y: 154 },
                                        { id: "Start", x: 212, y: 126 },
                                        { id: "RightStick", x: 238, y: 188 },
                                        { id: "West", x: 266, y: 154 },
                                        { id: "North", x: 292, y: 128 },
                                        { id: "East", x: 318, y: 154 },
                                        { id: "South", x: 292, y: 180 }
                                    ]

                                    Canvas {
                                        id: controllerShellCanvas
                                        anchors.fill: parent
                                        anchors.margins: 8
                                        onWidthChanged: requestPaint()
                                        onHeightChanged: requestPaint()
                                        onPaint: {
                                            const ctx = getContext("2d")
                                            ctx.reset()
                                            ctx.scale(width / 380, height / 240)
                                            ctx.beginPath()
                                            ctx.moveTo(73, 78)
                                            ctx.bezierCurveTo(91, 58, 127, 62, 151, 84)
                                            ctx.lineTo(229, 84)
                                            ctx.bezierCurveTo(253, 62, 289, 58, 307, 78)
                                            ctx.bezierCurveTo(332, 105, 343, 177, 324, 201)
                                            ctx.bezierCurveTo(309, 220, 283, 214, 258, 178)
                                            ctx.lineTo(122, 178)
                                            ctx.bezierCurveTo(97, 214, 71, 220, 56, 201)
                                            ctx.bezierCurveTo(37, 177, 48, 105, 73, 78)
                                            ctx.closePath()
                                            ctx.fillStyle = "#172331"
                                            ctx.fill()
                                            ctx.lineWidth = 2
                                            ctx.strokeStyle = "#34445a"
                                            ctx.stroke()
                                        }
                                    }

                                    Repeater {
                                        model: controllerDiagram.buttons
                                        delegate: RoundButton {
                                            required property var modelData
                                            width: 39
                                            height: 39
                                            x: 8 + (modelData.x / 380)
                                                     * (controllerDiagram.width - 16) - width / 2
                                            y: 8 + (modelData.y / 240)
                                                     * (controllerDiagram.height - 16) - height / 2
                                            z: 2
                                            text: {
                                                appSettings.controller_profile_revision
                                                appSettings.controller_profile_editor_layout
                                                return appSettings.controller_profile_button_label(
                                                    modelData.id)
                                            }
                                            font.pixelSize: text.length > 5 ? 7 : 9
                                            font.weight: Font.DemiBold
                                            onClicked: appSettings.select_controller_profile_target(
                                                           modelData.id)
                                            Accessible.name: "Map virtual button " + modelData.id
                                            background: Rectangle {
                                                radius: width / 2
                                                color: appSettings.controller_profile_editor_target
                                                       === modelData.id
                                                       ? root.accent : "#263447"
                                                border.color: appSettings.controller_profile_editor_target
                                                              === modelData.id
                                                              ? "#ffd092" : "#4a5b72"
                                                border.width: 1
                                            }
                                            contentItem: Text {
                                                text: parent.text
                                                color: appSettings.controller_profile_editor_target
                                                       === modelData.id
                                                       ? "#101318" : root.ink
                                                font: parent.font
                                                horizontalAlignment: Text.AlignHCenter
                                                verticalAlignment: Text.AlignVCenter
                                                elide: Text.ElideRight
                                            }
                                        }
                                    }
                                }

                                Rectangle {
                                    Layout.preferredWidth: 245
                                    Layout.fillHeight: true
                                    radius: 9
                                    color: "#141e2b"
                                    border.color: root.line

                                    ColumnLayout {
                                        anchors.fill: parent
                                        anchors.margins: 12
                                        spacing: 8
                                        Text {
                                            text: "VIRTUAL TARGET"
                                            color: root.muted
                                            font.pixelSize: 8
                                            font.weight: Font.Bold
                                            font.letterSpacing: 0.9
                                        }
                                        Text {
                                            Layout.fillWidth: true
                                            text: appSettings.controller_profile_button_label(
                                                      appSettings.controller_profile_editor_target)
                                            color: root.accent
                                            font.pixelSize: 17
                                            font.weight: Font.Bold
                                            elide: Text.ElideRight
                                        }
                                        Text {
                                            text: "Physical source"
                                            color: root.muted
                                            font.pixelSize: 9
                                        }
                                        ComboBox {
                                            id: physicalControllerSource
                                            Layout.fillWidth: true
                                            model: appSettings.controller_profile_button_count()
                                            delegate: ItemDelegate {
                                                required property int index
                                                width: physicalControllerSource.width
                                                text: appSettings.controller_profile_button_name_at(index)
                                            }
                                            currentIndex: {
                                                appSettings.controller_profile_revision
                                                const source = appSettings.controller_profile_selected_source()
                                                for (let i = 0;
                                                     i < appSettings.controller_profile_button_count();
                                                     ++i) {
                                                    if (appSettings.controller_profile_button_id_at(i)
                                                        === source)
                                                        return i
                                                }
                                                return 0
                                            }
                                            displayText: currentIndex >= 0
                                                         ? appSettings.controller_profile_button_name_at(
                                                               currentIndex) : "Choose source"
                                            onActivated: appSettings.choose_controller_profile_source(
                                                             appSettings.controller_profile_button_id_at(
                                                                 currentIndex))
                                            Accessible.name: "Physical source button"
                                        }
                                        Button {
                                            Layout.fillWidth: true
                                            text: "Apply 2-button X/A preset"
                                            onClicked: appSettings.apply_two_button_controller_preset()
                                        }
                                        Button {
                                            Layout.fillWidth: true
                                            text: "Clear all remaps"
                                            onClicked: appSettings.clear_controller_profile_mappings()
                                        }
                                        Text {
                                            text: "ACTIVE REMAPS"
                                            color: root.muted
                                            font.pixelSize: 8
                                            font.weight: Font.Bold
                                            font.letterSpacing: 0.9
                                        }
                                        ListView {
                                            id: controllerProfileMappingList
                                            Layout.fillWidth: true
                                            Layout.fillHeight: true
                                            clip: true
                                            spacing: 3
                                            model: {
                                                appSettings.controller_profile_revision
                                                return appSettings.controller_profile_mapping_count()
                                            }
                                            delegate: Rectangle {
                                                required property int index
                                                width: controllerProfileMappingList.width
                                                height: 25
                                                radius: 5
                                                color: "#1d2a39"
                                                Text {
                                                    anchors.centerIn: parent
                                                    text: appSettings.controller_profile_mapping_source_at(
                                                              index) + "  →  "
                                                          + appSettings.controller_profile_mapping_target_at(
                                                              index)
                                                    color: root.ink
                                                    font.pixelSize: 9
                                                }
                                            }
                                            Text {
                                                anchors.centerIn: parent
                                                visible: controllerProfileMappingList.count === 0
                                                text: "Identity mapping"
                                                color: root.muted
                                                font.pixelSize: 9
                                            }
                                        }
                                    }
                                }
                            }

                            Text {
                                Layout.fillWidth: true
                                text: appSettings.controller_profile_editor_status
                                color: appSettings.controller_profile_editor_status.indexOf(
                                           "must") >= 0
                                       || appSettings.controller_profile_editor_status.indexOf(
                                           "could not") >= 0
                                       ? "#ff8b8b" : root.muted
                                font.pixelSize: 9
                                wrapMode: Text.WordWrap
                            }

                            RowLayout {
                                Layout.fillWidth: true
                                Item { Layout.fillWidth: true }
                                Button {
                                    text: "Cancel"
                                    onClicked: appSettings.cancel_controller_profile_edit()
                                }
                                HeaderButton {
                                    text: "Save profile"
                                    active: true
                                    enabled: controllerProfileName.text.trim().length > 0
                                    onClicked: appSettings.save_controller_profile()
                                }
                            }
                        }
                    }
                }

                Rectangle { Layout.fillWidth: true; Layout.preferredHeight: 1; color: root.line }
                Text {
                    text: "EMULATORS & CORES"
                    color: root.accent
                    font.pixelSize: 10
                    font.weight: Font.Bold
                    font.letterSpacing: 1.2
                }
                RowLayout {
                    Layout.fillWidth: true
                    spacing: 14
                    ColumnLayout {
                        Layout.fillWidth: true
                        spacing: 3
                        Text {
                            text: emulatorManager.initialized
                                  ? emulatorManager.installed_count + " installed · "
                                    + emulatorManager.available_count + " available"
                                  : "Install and update emulators for this computer"
                            color: root.ink
                            font.pixelSize: 12
                            font.weight: Font.DemiBold
                        }
                        Text {
                            Layout.fillWidth: true
                            text: "Uses Flatpak, an isolated Nix profile, AppImage, GitHub releases, or direct downloads on Linux; winget on Windows; and Homebrew on macOS. Lunchbox only removes installations it created."
                            color: root.muted
                            font.pixelSize: 11
                            wrapMode: Text.WordWrap
                        }
                    }
                    ColumnLayout {
                        spacing: 7
                        HeaderButton {
                            Layout.fillWidth: true
                            text: "Manage emulators"
                            active: true
                            onClicked: {
                                settingsDialog.close()
                                root.openEmulatorManager()
                            }
                        }
                        Button {
                            Layout.fillWidth: true
                            text: "Launch commands"
                            onClicked: {
                                settingsDialog.close()
                                root.openLaunchProfileManager()
                            }
                        }
                    }
                }

                Rectangle {
                    Layout.fillWidth: true
                    Layout.preferredHeight: settingsMessage.implicitHeight + 22
                    radius: 9
                    color: appSettings.connection_ok ? "#172c28" : "#151d29"
                    border.color: appSettings.connection_ok ? root.accentCool : root.line
                    RowLayout {
                        anchors.fill: parent
                        anchors.margins: 11
                        BusyIndicator {
                            visible: appSettings.busy
                            running: visible
                            Layout.preferredWidth: 18
                            Layout.preferredHeight: 18
                        }
                        Text {
                            id: settingsMessage
                            Layout.fillWidth: true
                            text: appSettings.message
                            color: appSettings.connection_ok ? root.accentCool : root.muted
                            font.pixelSize: 11
                            wrapMode: Text.WordWrap
                        }
                    }
                }
                RowLayout {
                    Layout.fillWidth: true
                    Item { Layout.fillWidth: true }
                    Button {
                        text: "Test connection"
                        enabled: !appSettings.busy
                        onClicked: appSettings.test_connection()
                    }
                    HeaderButton {
                        text: "Save settings"
                        enabled: !appSettings.busy
                        active: true
                        onClicked: appSettings.save()
                    }
                }
                Text {
                    Layout.fillWidth: true
                    text: appSettings.state_database_path
                    color: "#566175"
                    font.pixelSize: 9
                    elide: Text.ElideMiddle
                    horizontalAlignment: Text.AlignRight
                }
            }
        }
    }

    Dialog {
        id: controllerProfileDeleteDialog
        parent: Overlay.overlay
        modal: true
        x: Math.round((parent.width - width) / 2)
        y: Math.round((parent.height - height) / 2)
        width: Math.min(470, root.width - 48)
        title: "Delete custom controller profile?"
        standardButtons: Dialog.Cancel | Dialog.Ok
        closePolicy: Popup.CloseOnEscape
        onAccepted: {
            appSettings.delete_controller_profile(
                root.pendingControllerProfileDeleteIndex)
            root.pendingControllerProfileDeleteIndex = -1
            root.pendingControllerProfileDeleteName = ""
        }
        onRejected: {
            root.pendingControllerProfileDeleteIndex = -1
            root.pendingControllerProfileDeleteName = ""
        }
        contentItem: Text {
            text: "Delete ‘" + root.pendingControllerProfileDeleteName
                  + "’? Lunchbox will also clear exact game, platform, default, and player assignments that reference it."
            color: root.ink
            font.pixelSize: 13
            wrapMode: Text.WordWrap
        }
    }

    Dialog {
        id: emulatorManagerDialog
        parent: Overlay.overlay
        modal: true
        x: Math.round((parent.width - width) / 2)
        y: Math.round((parent.height - height) / 2)
        width: Math.min(1040, root.width - 64)
        height: Math.min(800, root.height - 64)
        padding: 0
        closePolicy: Popup.CloseOnEscape

        background: Rectangle {
            color: root.panel
            radius: 14
            border.color: root.line
            border.width: 1
        }

        header: Rectangle {
            width: parent.width
            height: 72
            color: root.panelRaised
            radius: 14
            RowLayout {
                anchors.fill: parent
                anchors.leftMargin: 22
                anchors.rightMargin: 14
                spacing: 14
                ColumnLayout {
                    Layout.fillWidth: true
                    spacing: 1
                    Text {
                        text: "EMULATOR MANAGER"
                        color: root.ink
                        font.pixelSize: 17
                        font.weight: Font.Bold
                        font.letterSpacing: 0.8
                    }
                    Text {
                        text: "Native runtimes for this computer · no global Nix profile changes"
                        color: root.muted
                        font.pixelSize: 10
                    }
                }
                HeaderButton {
                    text: emulatorManager.busy ? "Refreshing…" : "↻  Refresh"
                    enabled: !emulatorManager.busy
                    onClicked: emulatorManager.refresh()
                }
                HeaderButton {
                    text: emulatorUpdates.initialized && emulatorUpdates.row_count > 0
                          ? emulatorUpdates.row_count + " updates" : "Check updates"
                    active: emulatorUpdates.initialized && emulatorUpdates.row_count > 0
                    enabled: !emulatorManager.busy && !emulatorUpdates.busy
                    onClicked: root.openEmulatorUpdates()
                }
                RoundButton {
                    text: "×"
                    flat: true
                    font.pixelSize: 20
                    onClicked: emulatorManagerDialog.close()
                }
            }
        }

        contentItem: ColumnLayout {
            spacing: 0

            Rectangle {
                Layout.fillWidth: true
                Layout.preferredHeight: 66
                color: "#101620"
                border.color: root.line
                RowLayout {
                    anchors.fill: parent
                    anchors.leftMargin: 20
                    anchors.rightMargin: 20
                    spacing: 11
                    TextField {
                        id: emulatorSearch
                        Layout.fillWidth: true
                        Layout.preferredHeight: 38
                        placeholderText: "Search emulators or package IDs"
                        selectByMouse: true
                        onTextEdited: emulatorManager.apply_filter(text,
                                                                    emulatorStatusFilter.currentValue)
                    }
                    ComboBox {
                        id: emulatorStatusFilter
                        Layout.preferredWidth: 176
                        textRole: "label"
                        valueRole: "value"
                        model: [
                            { label: "All runtimes", value: "all" },
                            { label: "Installed", value: "installed" },
                            { label: "Available", value: "available" },
                            { label: "Managed by Lunchbox", value: "managed" }
                        ]
                        onActivated: emulatorManager.apply_filter(emulatorSearch.text,
                                                                  currentValue)
                    }
                    Rectangle {
                        Layout.preferredWidth: installedSummary.implicitWidth + 22
                        Layout.preferredHeight: 30
                        radius: 15
                        color: "#172c28"
                        border.color: "#28584f"
                        Text {
                            id: installedSummary
                            anchors.centerIn: parent
                            text: emulatorManager.installed_count + " installed"
                            color: root.accentCool
                            font.pixelSize: 10
                            font.weight: Font.Bold
                        }
                    }
                    Rectangle {
                        Layout.preferredWidth: availableSummary.implicitWidth + 22
                        Layout.preferredHeight: 30
                        radius: 15
                        color: "#30291f"
                        border.color: "#5b4930"
                        Text {
                            id: availableSummary
                            anchors.centerIn: parent
                            text: emulatorManager.available_count + " available"
                            color: root.accent
                            font.pixelSize: 10
                            font.weight: Font.Bold
                        }
                    }
                }
            }

            Item {
                Layout.fillWidth: true
                Layout.fillHeight: true

                ListView {
                    id: emulatorList
                    anchors.fill: parent
                    anchors.leftMargin: 12
                    anchors.rightMargin: 12
                    anchors.topMargin: 10
                    anchors.bottomMargin: 10
                    clip: true
                    spacing: 7
                    reuseItems: true
                    model: emulatorManager.row_count
                    ScrollBar.vertical: ScrollBar { policy: ScrollBar.AsNeeded }

                    delegate: Rectangle {
                        id: emulatorRow
                        required property int index
                        readonly property int modelRevision: emulatorManager.revision
                        readonly property string emulatorName: emulatorManager.name_at(index)
                        readonly property string emulatorStatus: emulatorManager.status_at(index)
                        width: ListView.view.width
                        height: 78
                        radius: 10
                        color: emulatorHover.hovered ? "#1b2330" : "#151c27"
                        border.color: emulatorStatus === "MANAGED"
                                      ? "#28584f" : root.line

                        HoverHandler { id: emulatorHover }

                        RowLayout {
                            anchors.fill: parent
                            anchors.leftMargin: 17
                            anchors.rightMargin: 14
                            spacing: 14
                            Rectangle {
                                Layout.preferredWidth: 42
                                Layout.preferredHeight: 42
                                radius: 11
                                color: emulatorRow.emulatorStatus === "MANAGED"
                                       || emulatorRow.emulatorStatus === "INSTALLED"
                                       ? "#1b3430" : "#292a30"
                                Text {
                                    anchors.centerIn: parent
                                    text: emulatorRow.emulatorName.length > 0
                                          ? emulatorRow.emulatorName.charAt(0).toUpperCase() : "?"
                                    color: emulatorRow.emulatorStatus === "MANAGED"
                                           || emulatorRow.emulatorStatus === "INSTALLED"
                                           ? root.accentCool : root.accent
                                    font.pixelSize: 18
                                    font.weight: Font.Black
                                }
                            }
                            ColumnLayout {
                                Layout.fillWidth: true
                                spacing: 3
                                RowLayout {
                                    Layout.fillWidth: true
                                    spacing: 9
                                    Text {
                                        text: emulatorRow.emulatorName
                                        color: root.ink
                                        font.pixelSize: 13
                                        font.weight: Font.DemiBold
                                    }
                                    Rectangle {
                                        Layout.preferredWidth: sourceText.implicitWidth + 14
                                        Layout.preferredHeight: 20
                                        radius: 6
                                        color: "#202938"
                                        border.color: root.line
                                        Text {
                                            id: sourceText
                                            anchors.centerIn: parent
                                            text: emulatorManager.source_at(emulatorRow.index)
                                            color: "#b7c2d2"
                                            font.pixelSize: 9
                                            font.weight: Font.Bold
                                        }
                                    }
                                    Item { Layout.fillWidth: true }
                                }
                                Text {
                                    Layout.fillWidth: true
                                    text: emulatorManager.detail_at(emulatorRow.index)
                                    color: root.muted
                                    font.pixelSize: 10
                                    elide: Text.ElideRight
                                }
                            }
                            Text {
                                Layout.preferredWidth: 142
                                text: emulatorRow.emulatorStatus
                                color: emulatorRow.emulatorStatus === "MANAGED"
                                       ? root.accentCool
                                       : emulatorRow.emulatorStatus === "INSTALLED"
                                         ? "#b7c2d2" : root.accent
                                horizontalAlignment: Text.AlignRight
                                font.pixelSize: 9
                                font.weight: Font.Bold
                                font.letterSpacing: 0.6
                            }
                            BusyIndicator {
                                visible: emulatorManager.busy
                                         && emulatorManager.busy_index === emulatorRow.index
                                running: visible
                                Layout.preferredWidth: 26
                                Layout.preferredHeight: 26
                            }
                            HeaderButton {
                                visible: !emulatorManager.busy
                                         && emulatorManager.can_install_at(emulatorRow.index)
                                text: "Install"
                                active: true
                                onClicked: emulatorManager.install_at(emulatorRow.index)
                            }
                            Button {
                                visible: !emulatorManager.busy
                                         && emulatorManager.can_uninstall_at(emulatorRow.index)
                                text: "Remove"
                                onClicked: {
                                    root.pendingEmulatorUninstallIndex = emulatorRow.index
                                    root.pendingEmulatorUninstallName = emulatorRow.emulatorName
                                    emulatorUninstallDialog.open()
                                }
                            }
                        }
                    }
                }

                Column {
                    anchors.centerIn: parent
                    spacing: 10
                    visible: emulatorManager.busy && !emulatorManager.initialized
                    BusyIndicator {
                        anchors.horizontalCenter: parent.horizontalCenter
                        running: parent.visible
                    }
                    Text {
                        text: "Detecting installed emulators…"
                        color: root.muted
                        font.pixelSize: 12
                    }
                }

                Column {
                    anchors.centerIn: parent
                    width: Math.min(430, parent.width - 80)
                    spacing: 7
                    visible: emulatorManager.initialized && !emulatorManager.busy
                             && emulatorManager.row_count === 0
                    Text {
                        width: parent.width
                        text: "No matching emulators"
                        color: root.ink
                        font.pixelSize: 15
                        font.weight: Font.DemiBold
                        horizontalAlignment: Text.AlignHCenter
                    }
                    Text {
                        width: parent.width
                        text: "Try another search or status filter."
                        color: root.muted
                        font.pixelSize: 11
                        horizontalAlignment: Text.AlignHCenter
                    }
                }
            }

            Rectangle {
                Layout.fillWidth: true
                Layout.preferredHeight: 54
                color: "#101620"
                border.color: root.line
                RowLayout {
                    anchors.fill: parent
                    anchors.leftMargin: 20
                    anchors.rightMargin: 20
                    spacing: 10
                    BusyIndicator {
                        visible: emulatorManager.busy
                        running: visible
                        Layout.preferredWidth: 20
                        Layout.preferredHeight: 20
                    }
                    Text {
                        Layout.fillWidth: true
                        text: emulatorManager.message
                        color: emulatorManager.message.indexOf("failed") >= 0
                               || emulatorManager.message.indexOf("Could not") >= 0
                               ? "#ff8c82" : root.muted
                        font.pixelSize: 10
                        elide: Text.ElideRight
                    }
                    Text {
                        text: "External installs are never removed"
                        color: "#657186"
                        font.pixelSize: 9
                    }
                }
            }
        }
    }

    Dialog {
        id: emulatorUpdateDialog
        parent: Overlay.overlay
        modal: true
        x: Math.round((parent.width - width) / 2)
        y: Math.round((parent.height - height) / 2)
        width: Math.min(920, root.width - 80)
        height: Math.min(720, root.height - 80)
        padding: 0
        closePolicy: emulatorUpdates.busy ? Popup.NoAutoClose : Popup.CloseOnEscape
        onClosed: {
            if (emulatorManager.initialized && !emulatorManager.busy)
                emulatorManager.refresh()
        }

        background: Rectangle {
            color: root.panel
            radius: 14
            border.color: root.line
            border.width: 1
        }

        header: Rectangle {
            width: parent.width
            height: 82
            color: root.panelRaised
            radius: 14
            RowLayout {
                anchors.fill: parent
                anchors.leftMargin: 22
                anchors.rightMargin: 14
                spacing: 14
                Rectangle {
                    Layout.preferredWidth: 44
                    Layout.preferredHeight: 44
                    radius: 12
                    color: "#253229"
                    border.color: "#3b654f"
                    Text {
                        anchors.centerIn: parent
                        text: "↥"
                        color: root.accentCool
                        font.pixelSize: 23
                        font.weight: Font.Bold
                    }
                }
                ColumnLayout {
                    Layout.fillWidth: true
                    spacing: 2
                    Text {
                        text: "EMULATOR UPDATES"
                        color: root.ink
                        font.pixelSize: 17
                        font.weight: Font.Bold
                        font.letterSpacing: 0.8
                    }
                    Text {
                        text: "Review exact version changes before updating any runtime"
                        color: root.muted
                        font.pixelSize: 10
                    }
                }
                Rectangle {
                    visible: emulatorUpdates.initialized
                    Layout.preferredWidth: updateCountText.implicitWidth + 22
                    Layout.preferredHeight: 30
                    radius: 15
                    color: emulatorUpdates.row_count > 0 ? "#30291f" : "#172c28"
                    border.color: emulatorUpdates.row_count > 0 ? "#5b4930" : "#28584f"
                    Text {
                        id: updateCountText
                        anchors.centerIn: parent
                        text: emulatorUpdates.row_count > 0
                              ? emulatorUpdates.row_count + " available" : "up to date"
                        color: emulatorUpdates.row_count > 0 ? root.accent : root.accentCool
                        font.pixelSize: 10
                        font.weight: Font.Bold
                    }
                }
                RoundButton {
                    text: "×"
                    flat: true
                    enabled: !emulatorUpdates.busy
                    font.pixelSize: 20
                    onClicked: emulatorUpdateDialog.close()
                }
            }
        }

        contentItem: Rectangle {
            id: emulatorUpdatePanel
            color: root.panel

            ColumnLayout {
                anchors.fill: parent
                spacing: 0

                Rectangle {
                    Layout.fillWidth: true
                    Layout.preferredHeight: 52
                    color: "#101620"
                    border.color: root.line
                    RowLayout {
                        anchors.fill: parent
                        anchors.leftMargin: 18
                        anchors.rightMargin: 18
                        spacing: 10
                        HeaderButton {
                            text: emulatorUpdates.selected_count === emulatorUpdates.row_count
                                  && emulatorUpdates.row_count > 0
                                  ? "Clear selection" : "Select all"
                            enabled: !emulatorUpdates.busy && emulatorUpdates.row_count > 0
                            onClicked: emulatorUpdates.select_all(
                                           emulatorUpdates.selected_count !== emulatorUpdates.row_count)
                        }
                        Text {
                            text: emulatorUpdates.selected_count + " selected"
                            color: root.muted
                            font.pixelSize: 10
                        }
                        Item { Layout.fillWidth: true }
                        Text {
                            text: "Updates run only when requested here"
                            color: "#657186"
                            font.pixelSize: 9
                        }
                    }
                }

                Item {
                    Layout.fillWidth: true
                    Layout.fillHeight: true

                    ListView {
                        id: emulatorUpdateList
                        anchors.fill: parent
                        anchors.margins: 13
                        clip: true
                        spacing: 8
                        reuseItems: true
                        model: emulatorUpdates.row_count
                        ScrollBar.vertical: ScrollBar { policy: ScrollBar.AsNeeded }

                        delegate: Rectangle {
                            id: emulatorUpdateRow
                            required property int index
                            readonly property int modelRevision: emulatorUpdates.revision
                            readonly property string updateStatus: {
                                modelRevision
                                return emulatorUpdates.status_at(index)
                            }
                            readonly property string updateDetail: {
                                modelRevision
                                return emulatorUpdates.status_detail_at(index)
                            }
                            width: ListView.view.width
                            height: updateDetail.length > 0 ? 98 : 80
                            radius: 10
                            color: updateHover.hovered ? "#1b2330" : "#151c27"
                            border.color: updateStatus === "UPDATED" ? "#28584f"
                                          : updateStatus === "FAILED" ? "#713c43" : root.line

                            HoverHandler { id: updateHover }

                            RowLayout {
                                anchors.fill: parent
                                anchors.leftMargin: 13
                                anchors.rightMargin: 15
                                spacing: 12
                                CheckBox {
                                    checked: {
                                        emulatorUpdateRow.modelRevision
                                        return emulatorUpdates.selected_at(emulatorUpdateRow.index)
                                    }
                                    enabled: !emulatorUpdates.busy
                                             && (emulatorUpdateRow.updateStatus === "PENDING"
                                                 || emulatorUpdateRow.updateStatus === "FAILED")
                                    onToggled: {
                                        const saved = emulatorUpdates.selected_at(
                                                          emulatorUpdateRow.index)
                                        if (checked !== saved)
                                            emulatorUpdates.set_selected(
                                                emulatorUpdateRow.index, checked)
                                    }
                                    Accessible.name: "Select "
                                                     + emulatorUpdates.name_at(
                                                         emulatorUpdateRow.index)
                                }
                                Rectangle {
                                    Layout.preferredWidth: 42
                                    Layout.preferredHeight: 42
                                    radius: 11
                                    color: emulatorUpdateRow.updateStatus === "UPDATED"
                                           ? "#1b3430" : "#292a30"
                                    Text {
                                        anchors.centerIn: parent
                                        text: "↥"
                                        color: emulatorUpdateRow.updateStatus === "UPDATED"
                                               ? root.accentCool : root.accent
                                        font.pixelSize: 18
                                        font.weight: Font.Black
                                    }
                                }
                                ColumnLayout {
                                    Layout.fillWidth: true
                                    spacing: 3
                                    RowLayout {
                                        Layout.fillWidth: true
                                        spacing: 9
                                        Text {
                                            Layout.fillWidth: true
                                            text: emulatorUpdates.name_at(emulatorUpdateRow.index)
                                            color: root.ink
                                            font.pixelSize: 13
                                            font.weight: Font.DemiBold
                                            elide: Text.ElideRight
                                        }
                                        Rectangle {
                                            Layout.preferredWidth: updateSource.implicitWidth + 14
                                            Layout.preferredHeight: 20
                                            radius: 6
                                            color: "#202938"
                                            border.color: root.line
                                            Text {
                                                id: updateSource
                                                anchors.centerIn: parent
                                                text: emulatorUpdates.source_at(
                                                          emulatorUpdateRow.index)
                                                color: "#b7c2d2"
                                                font.pixelSize: 9
                                                font.weight: Font.Bold
                                            }
                                        }
                                    }
                                    Text {
                                        Layout.fillWidth: true
                                        text: emulatorUpdates.version_at(emulatorUpdateRow.index)
                                        color: root.accentCool
                                        font.pixelSize: 11
                                        font.weight: Font.DemiBold
                                        elide: Text.ElideRight
                                    }
                                    Text {
                                        Layout.fillWidth: true
                                        visible: emulatorUpdateRow.updateDetail.length > 0
                                        text: emulatorUpdateRow.updateDetail
                                        color: emulatorUpdateRow.updateStatus === "FAILED"
                                               ? "#ff8c82" : root.muted
                                        font.pixelSize: 9
                                        elide: Text.ElideRight
                                    }
                                }
                                BusyIndicator {
                                    visible: emulatorUpdateRow.updateStatus === "UPDATING"
                                    running: visible
                                    Layout.preferredWidth: 24
                                    Layout.preferredHeight: 24
                                }
                                Text {
                                    Layout.preferredWidth: 76
                                    text: emulatorUpdateRow.updateStatus
                                    color: emulatorUpdateRow.updateStatus === "UPDATED"
                                           ? root.accentCool
                                           : emulatorUpdateRow.updateStatus === "FAILED"
                                             ? "#ff8c82" : root.accent
                                    horizontalAlignment: Text.AlignRight
                                    font.pixelSize: 9
                                    font.weight: Font.Bold
                                    font.letterSpacing: 0.5
                                }
                            }
                        }
                    }

                    Column {
                        anchors.centerIn: parent
                        width: Math.min(430, parent.width - 80)
                        spacing: 10
                        visible: emulatorUpdates.busy && !emulatorUpdates.initialized
                        BusyIndicator {
                            anchors.horizontalCenter: parent.horizontalCenter
                            running: parent.visible
                        }
                        Text {
                            width: parent.width
                            text: "Checking package sources…"
                            color: root.muted
                            font.pixelSize: 12
                            horizontalAlignment: Text.AlignHCenter
                        }
                    }

                    Column {
                        anchors.centerIn: parent
                        width: Math.min(460, parent.width - 80)
                        spacing: 8
                        visible: emulatorUpdates.initialized && !emulatorUpdates.busy
                                 && emulatorUpdates.row_count === 0
                        Text {
                            width: parent.width
                            text: "Everything is current"
                            color: root.ink
                            font.pixelSize: 17
                            font.weight: Font.DemiBold
                            horizontalAlignment: Text.AlignHCenter
                        }
                        Text {
                            width: parent.width
                            text: "No supported emulator installation has a newer verified build."
                            color: root.muted
                            font.pixelSize: 11
                            wrapMode: Text.WordWrap
                            horizontalAlignment: Text.AlignHCenter
                        }
                    }
                }

                Rectangle {
                    Layout.fillWidth: true
                    Layout.preferredHeight: 66
                    color: "#101620"
                    border.color: root.line
                    RowLayout {
                        anchors.fill: parent
                        anchors.leftMargin: 18
                        anchors.rightMargin: 18
                        spacing: 10
                        BusyIndicator {
                            visible: emulatorUpdates.busy
                            running: visible
                            Layout.preferredWidth: 20
                            Layout.preferredHeight: 20
                        }
                        ColumnLayout {
                            Layout.fillWidth: true
                            spacing: 1
                            Text {
                                Layout.fillWidth: true
                                text: emulatorUpdates.message
                                color: emulatorUpdates.message.indexOf("failed") >= 0
                                       || emulatorUpdates.message.indexOf("Could not") >= 0
                                       ? "#ff8c82" : root.muted
                                font.pixelSize: 10
                                elide: Text.ElideRight
                            }
                            Text {
                                visible: emulatorUpdates.busy && emulatorUpdates.row_count > 0
                                text: emulatorUpdates.completed_count + " of "
                                      + emulatorUpdates.batch_count + " completed"
                                color: "#657186"
                                font.pixelSize: 9
                            }
                        }
                        HeaderButton {
                            text: emulatorUpdates.busy ? "Checking…" : "↻  Refresh"
                            enabled: !emulatorUpdates.busy
                            onClicked: emulatorUpdates.refresh()
                        }
                        HeaderButton {
                            text: emulatorUpdates.busy
                                  ? "Updating…" : "Update selected ("
                                    + emulatorUpdates.selected_count + ")"
                            active: true
                            enabled: !emulatorUpdates.busy
                                     && emulatorUpdates.selected_count > 0
                            onClicked: emulatorUpdates.update_selected()
                        }
                    }
                }
            }
        }
    }

    Timer {
        id: emulatorUpdateScreenshotTimer
        interval: 900
        repeat: false
        onTriggered: {
            if (root.screenshotOutput.length === 0)
                return
            emulatorUpdatePanel.grabToImage(function(result) {
                if (!result.saveToFile(root.screenshotOutput)) {
                    console.error("LUNCHBOX_EMULATOR_UPDATE_UI_FAILED screenshot="
                                  + root.screenshotOutput)
                    Qt.exit(2)
                    return
                }
                console.log("LUNCHBOX_EMULATOR_UPDATE_UI_READY updates="
                            + emulatorUpdates.row_count + " screenshot="
                            + root.screenshotOutput)
                Qt.quit()
            })
        }
    }

    Dialog {
        id: launchProfileManagerDialog
        parent: Overlay.overlay
        modal: true
        x: Math.round((parent.width - width) / 2)
        y: Math.round((parent.height - height) / 2)
        width: Math.min(1240, root.width - 64)
        height: Math.min(880, root.height - 64)
        padding: 0
        closePolicy: Popup.CloseOnEscape

        background: Rectangle {
            color: root.panel
            radius: 14
            border.color: root.line
            border.width: 1
        }

        header: Rectangle {
            width: parent.width
            height: 78
            color: root.panelRaised
            radius: 14
            RowLayout {
                anchors.fill: parent
                anchors.leftMargin: 22
                anchors.rightMargin: 14
                spacing: 14
                ColumnLayout {
                    Layout.fillWidth: true
                    spacing: 2
                    Text {
                        text: "LAUNCH COMMANDS"
                        color: root.ink
                        font.pixelSize: 17
                        font.weight: Font.Bold
                        font.letterSpacing: 0.8
                    }
                    Text {
                        text: "Exact emulator/runtime profiles · game overrides remain in Game Details"
                        color: root.muted
                        font.pixelSize: 10
                    }
                }
                HeaderButton {
                    text: launchProfileManager.busy ? "Refreshing…" : "↻  Refresh"
                    enabled: !launchProfileManager.busy
                    onClicked: launchProfileManager.refresh()
                }
                RoundButton {
                    text: "×"
                    flat: true
                    font.pixelSize: 20
                    onClicked: launchProfileManagerDialog.close()
                }
            }
        }

        contentItem: ColumnLayout {
            id: launchProfileManagerPanel
            spacing: 0

            Rectangle {
                Layout.fillWidth: true
                Layout.preferredHeight: 72
                color: "#101620"
                border.color: root.line
                RowLayout {
                    anchors.fill: parent
                    anchors.leftMargin: 18
                    anchors.rightMargin: 18
                    spacing: 10
                    TextField {
                        id: launchProfileSearch
                        Layout.fillWidth: true
                        Layout.preferredHeight: 38
                        placeholderText: "Search platform, emulator, core, argument, or template"
                        selectByMouse: true
                        color: root.ink
                        placeholderTextColor: root.muted
                        selectionColor: root.accent
                        selectedTextColor: "#101318"
                        background: Rectangle {
                            radius: 7
                            color: "#0b121b"
                            border.color: launchProfileSearch.activeFocus
                                          ? root.accent : root.line
                        }
                        onTextEdited: launchProfileManager.apply_filter(
                                          text,
                                          launchProfileScopeFilter.currentValue,
                                          launchProfileCustomizationFilter.currentValue)
                    }
                    ComboBox {
                        id: launchProfileScopeFilter
                        Layout.preferredWidth: 166
                        textRole: "label"
                        valueRole: "value"
                        model: [
                            { label: "All scopes", value: "all" },
                            { label: "All platforms", value: "global" },
                            { label: "Per platform", value: "platform" }
                        ]
                        contentItem: Text {
                            leftPadding: 10
                            rightPadding: 28
                            text: launchProfileScopeFilter.displayText
                            color: root.ink
                            font.pixelSize: 10
                            verticalAlignment: Text.AlignVCenter
                            elide: Text.ElideRight
                        }
                        background: Rectangle {
                            radius: 7
                            color: "#0b121b"
                            border.color: launchProfileScopeFilter.activeFocus
                                          ? root.accent : root.line
                        }
                        onActivated: launchProfileManager.apply_filter(
                                         launchProfileSearch.text, currentValue,
                                         launchProfileCustomizationFilter.currentValue)
                    }
                    ComboBox {
                        id: launchProfileCustomizationFilter
                        Layout.preferredWidth: 166
                        textRole: "label"
                        valueRole: "value"
                        model: [
                            { label: "All profiles", value: "all" },
                            { label: "Customized", value: "customized" },
                            { label: "Built-in", value: "built-in" }
                        ]
                        contentItem: Text {
                            leftPadding: 10
                            rightPadding: 28
                            text: launchProfileCustomizationFilter.displayText
                            color: root.ink
                            font.pixelSize: 10
                            verticalAlignment: Text.AlignVCenter
                            elide: Text.ElideRight
                        }
                        background: Rectangle {
                            radius: 7
                            color: "#0b121b"
                            border.color: launchProfileCustomizationFilter.activeFocus
                                          ? root.accent : root.line
                        }
                        onActivated: launchProfileManager.apply_filter(
                                         launchProfileSearch.text,
                                         launchProfileScopeFilter.currentValue,
                                         currentValue)
                    }
                    Rectangle {
                        Layout.preferredWidth: launchProfileCount.implicitWidth + 22
                        Layout.preferredHeight: 30
                        radius: 15
                        color: "#172c28"
                        border.color: "#28584f"
                        Text {
                            id: launchProfileCount
                            anchors.centerIn: parent
                            text: launchProfileManager.row_count + " shown · "
                                  + launchProfileManager.customized_count + " custom"
                            color: root.accentCool
                            font.pixelSize: 10
                            font.weight: Font.Bold
                        }
                    }
                }
            }

            RowLayout {
                Layout.fillWidth: true
                Layout.fillHeight: true
                spacing: 0

                Rectangle {
                    Layout.fillWidth: true
                    Layout.fillHeight: true
                    color: root.panel

                    ColumnLayout {
                        anchors.fill: parent
                        spacing: 0
                        Rectangle {
                            Layout.fillWidth: true
                            Layout.preferredHeight: 34
                            color: "#0e141d"
                            RowLayout {
                                anchors.fill: parent
                                anchors.leftMargin: 16
                                anchors.rightMargin: 16
                                spacing: 12
                                Text {
                                    Layout.preferredWidth: 164
                                    text: "SCOPE"
                                    color: root.muted
                                    font.pixelSize: 9
                                    font.weight: Font.Bold
                                }
                                Text {
                                    Layout.preferredWidth: 174
                                    text: "EMULATOR"
                                    color: root.muted
                                    font.pixelSize: 9
                                    font.weight: Font.Bold
                                }
                                Text {
                                    Layout.preferredWidth: 145
                                    text: "RUNTIME"
                                    color: root.muted
                                    font.pixelSize: 9
                                    font.weight: Font.Bold
                                }
                                Text {
                                    Layout.fillWidth: true
                                    text: "BUILT-IN / STATUS"
                                    color: root.muted
                                    font.pixelSize: 9
                                    font.weight: Font.Bold
                                }
                            }
                        }
                        ListView {
                            id: launchProfileList
                            Layout.fillWidth: true
                            Layout.fillHeight: true
                            clip: true
                            reuseItems: true
                            spacing: 1
                            model: launchProfileManager.row_count
                            ScrollBar.vertical: ScrollBar { policy: ScrollBar.AsNeeded }

                            delegate: Rectangle {
                                id: launchProfileRow
                                required property int index
                                readonly property int modelRevision: launchProfileManager.revision
                                readonly property bool customized:
                                    launchProfileManager.customized_at(index)
                                width: ListView.view.width
                                height: 66
                                color: launchProfileRowMouse.containsMouse
                                       ? "#1b2532" : index % 2 === 0
                                         ? "#141c27" : "#111923"
                                border.color: customized ? "#28584f" : "transparent"

                                RowLayout {
                                    anchors.fill: parent
                                    anchors.leftMargin: 16
                                    anchors.rightMargin: 16
                                    spacing: 12
                                    Text {
                                        Layout.preferredWidth: 164
                                        text: launchProfileManager.scope_at(
                                                  launchProfileRow.index)
                                        color: root.ink
                                        font.pixelSize: 11
                                        font.weight: Font.DemiBold
                                        elide: Text.ElideRight
                                    }
                                    Text {
                                        Layout.preferredWidth: 174
                                        text: launchProfileManager.emulator_at(
                                                  launchProfileRow.index)
                                        color: root.ink
                                        font.pixelSize: 11
                                        elide: Text.ElideRight
                                    }
                                    Text {
                                        Layout.preferredWidth: 145
                                        text: launchProfileManager.runtime_at(
                                                  launchProfileRow.index)
                                        color: "#b7c2d2"
                                        font.pixelSize: 10
                                        elide: Text.ElideRight
                                    }
                                    ColumnLayout {
                                        Layout.fillWidth: true
                                        spacing: 2
                                        Text {
                                            Layout.fillWidth: true
                                            text: launchProfileManager.default_template_at(
                                                      launchProfileRow.index)
                                            color: root.muted
                                            font.family: "monospace"
                                            font.pixelSize: 9
                                            elide: Text.ElideRight
                                        }
                                        Text {
                                            text: launchProfileManager.customization_at(
                                                      launchProfileRow.index)
                                            color: launchProfileRow.customized
                                                   ? root.accentCool : "#657186"
                                            font.pixelSize: 9
                                            font.weight: Font.Bold
                                        }
                                    }
                                }
                                MouseArea {
                                    id: launchProfileRowMouse
                                    anchors.fill: parent
                                    hoverEnabled: true
                                    cursorShape: Qt.PointingHandCursor
                                    onClicked: launchProfileManager.edit_at(
                                                   launchProfileRow.index)
                                }
                            }

                            Column {
                                anchors.centerIn: parent
                                spacing: 8
                                visible: launchProfileManager.initialized
                                         && !launchProfileManager.busy
                                         && launchProfileManager.row_count === 0
                                Text {
                                    anchors.horizontalCenter: parent.horizontalCenter
                                    text: "No matching launch profiles"
                                    color: root.ink
                                    font.pixelSize: 14
                                    font.weight: Font.DemiBold
                                }
                                Text {
                                    anchors.horizontalCenter: parent.horizontalCenter
                                    text: "Try another search or filter."
                                    color: root.muted
                                    font.pixelSize: 10
                                }
                            }
                        }
                    }

                    Column {
                        anchors.centerIn: parent
                        spacing: 10
                        visible: launchProfileManager.busy
                                 && !launchProfileManager.initialized
                        BusyIndicator {
                            anchors.horizontalCenter: parent.horizontalCenter
                            running: parent.visible
                        }
                        Text {
                            text: "Loading exact runtime profiles…"
                            color: root.muted
                            font.pixelSize: 11
                        }
                    }
                }

                Rectangle {
                    Layout.preferredWidth: 410
                    Layout.fillHeight: true
                    color: "#101720"
                    border.color: root.line

                    ColumnLayout {
                        anchors.fill: parent
                        anchors.margins: 18
                        spacing: 10
                        visible: launchProfileManager.editor_open
                        Text {
                            Layout.fillWidth: true
                            text: launchProfileManager.editor_title
                            color: root.ink
                            font.pixelSize: 15
                            font.weight: Font.Bold
                            elide: Text.ElideRight
                        }
                        Text {
                            Layout.fillWidth: true
                            text: launchProfileManager.editor_identity
                            color: root.muted
                            font.pixelSize: 9
                            wrapMode: Text.WordWrap
                        }
                        Rectangle {
                            Layout.fillWidth: true
                            Layout.preferredHeight: 54
                            radius: 8
                            color: "#0b121b"
                            border.color: root.line
                            Column {
                                anchors.fill: parent
                                anchors.margins: 9
                                spacing: 3
                                Text {
                                    text: "BUILT-IN"
                                    color: root.muted
                                    font.pixelSize: 8
                                    font.weight: Font.Bold
                                }
                                Text {
                                    width: parent.width
                                    text: launchProfileManager.editor_default_template
                                    color: "#aeb9c9"
                                    font.family: "monospace"
                                    font.pixelSize: 9
                                    elide: Text.ElideRight
                                }
                            }
                        }
                        Text {
                            Layout.fillWidth: true
                            text: launchProfileManager.editor_effective_summary
                            color: root.accentCool
                            font.pixelSize: 9
                            wrapMode: Text.WordWrap
                        }
                        Text {
                            text: "EXTRA ARGUMENTS"
                            color: root.muted
                            font.pixelSize: 8
                            font.weight: Font.Bold
                        }
                        TextField {
                            id: launchProfileManagerExtraArguments
                            Layout.fillWidth: true
                            placeholderText: "Example: --fullscreen --latency 1"
                            selectByMouse: true
                            color: root.ink
                            placeholderTextColor: root.muted
                            selectionColor: root.accent
                            selectedTextColor: "#101318"
                            background: Rectangle {
                                radius: 7
                                color: "#0b121b"
                                border.color: launchProfileManagerExtraArguments.activeFocus
                                              ? root.accent : root.line
                            }
                        }
                        Text {
                            text: "COMMAND TEMPLATE (OPTIONAL)"
                            color: root.muted
                            font.pixelSize: 8
                            font.weight: Font.Bold
                        }
                        ScrollView {
                            Layout.fillWidth: true
                            Layout.preferredHeight: 128
                            TextArea {
                                id: launchProfileManagerCommandTemplate
                                placeholderText: "Leave blank for the built-in template"
                                selectByMouse: true
                                wrapMode: TextEdit.Wrap
                                color: root.ink
                                placeholderTextColor: root.muted
                                selectionColor: root.accent
                                selectedTextColor: "#101318"
                                background: Rectangle {
                                    radius: 7
                                    color: "#0b121b"
                                    border.color: launchProfileManagerCommandTemplate.activeFocus
                                                  ? root.accent : root.line
                                }
                            }
                        }
                        Text {
                            Layout.fillWidth: true
                            text: "Quotes group arguments. Lunchbox parses directly to argv; it never invokes a shell. A saved template replaces the built-in command, while extra arguments augment it only when the template is blank."
                            color: root.muted
                            font.pixelSize: 9
                            wrapMode: Text.WordWrap
                        }
                        Text {
                            Layout.fillWidth: true
                            text: launchProfileManager.editor_status
                            color: launchProfileManager.editor_status.indexOf(
                                       "Could not") === 0 ? "#ff8b8b" : root.accentCool
                            font.pixelSize: 9
                            wrapMode: Text.WordWrap
                            visible: text.length > 0
                        }
                        Item { Layout.fillHeight: true }
                        RowLayout {
                            Layout.fillWidth: true
                            spacing: 8
                            Button {
                                text: "Clear profile"
                                onClicked: launchProfileManager.clear_editor()
                                contentItem: Text {
                                    text: parent.text
                                    color: root.ink
                                    font.pixelSize: 10
                                    font.weight: Font.DemiBold
                                    horizontalAlignment: Text.AlignHCenter
                                    verticalAlignment: Text.AlignVCenter
                                }
                                background: Rectangle {
                                    radius: 7
                                    color: parent.down ? "#3a2029" : "#251a21"
                                    border.color: "#70404d"
                                }
                            }
                            Item { Layout.fillWidth: true }
                            HeaderButton {
                                text: "Save profile"
                                active: true
                                onClicked: launchProfileManager.save_editor(
                                               launchProfileManagerExtraArguments.text,
                                               launchProfileManagerCommandTemplate.text)
                            }
                        }
                    }

                    Column {
                        anchors.centerIn: parent
                        width: parent.width - 64
                        spacing: 8
                        visible: !launchProfileManager.editor_open
                        Text {
                            width: parent.width
                            text: "Choose a profile"
                            color: root.ink
                            font.pixelSize: 15
                            font.weight: Font.DemiBold
                            horizontalAlignment: Text.AlignHCenter
                        }
                        Text {
                            width: parent.width
                            text: "Edit exact all-platform or per-platform values. Per-game overrides stay beside Play in Game Details."
                            color: root.muted
                            font.pixelSize: 10
                            wrapMode: Text.WordWrap
                            horizontalAlignment: Text.AlignHCenter
                        }
                    }
                }
            }

            Rectangle {
                Layout.fillWidth: true
                Layout.preferredHeight: 54
                color: "#0e141d"
                border.color: root.line
                RowLayout {
                    anchors.fill: parent
                    anchors.leftMargin: 18
                    anchors.rightMargin: 18
                    spacing: 10
                    BusyIndicator {
                        visible: launchProfileManager.busy
                        running: visible
                        Layout.preferredWidth: 20
                        Layout.preferredHeight: 20
                    }
                    Text {
                        Layout.fillWidth: true
                        text: launchProfileManager.message
                        color: launchProfileManager.message.indexOf("Could not") >= 0
                               ? "#ff8b8b" : root.muted
                        font.pixelSize: 10
                        elide: Text.ElideRight
                    }
                    Text {
                        text: "Stable emulator IDs · no shell"
                        color: "#657186"
                        font.pixelSize: 9
                    }
                }
            }
        }
    }

    Timer {
        id: launchProfileManagerScreenshotTimer
        interval: 450
        repeat: false
        onTriggered: {
            if (root.screenshotOutput.length === 0)
                return
            launchProfileManagerPanel.grabToImage(function(result) {
                if (!result.saveToFile(root.screenshotOutput)) {
                    console.error("LUNCHBOX_LAUNCH_PROFILE_MANAGER_SCREENSHOT_FAILED path="
                                  + root.screenshotOutput)
                    Qt.exit(2)
                    return
                }
                console.log("LUNCHBOX_LAUNCH_PROFILE_MANAGER_SCREENSHOT_READY path="
                            + root.screenshotOutput)
                Qt.quit()
            })
        }
    }

    Dialog {
        id: emulatorUninstallDialog
        parent: Overlay.overlay
        modal: true
        x: Math.round((parent.width - width) / 2)
        y: Math.round((parent.height - height) / 2)
        width: 430
        title: "Remove " + root.pendingEmulatorUninstallName + "?"
        standardButtons: Dialog.Cancel | Dialog.Ok
        onAccepted: {
            if (root.pendingEmulatorUninstallIndex >= 0)
                emulatorManager.uninstall_at(root.pendingEmulatorUninstallIndex)
            root.pendingEmulatorUninstallIndex = -1
            root.pendingEmulatorUninstallName = ""
        }
        onRejected: {
            root.pendingEmulatorUninstallIndex = -1
            root.pendingEmulatorUninstallName = ""
        }
        contentItem: Text {
            text: "Lunchbox will remove only the installation it created. Your games, saves, firmware, and externally managed emulator installations are not touched."
            color: root.ink
            wrapMode: Text.WordWrap
            font.pixelSize: 12
        }
    }

    Drawer {
        id: downloadsDrawer
        edge: Qt.RightEdge
        width: Math.min(480, root.width * 0.42)
        height: root.height
        modal: false
        interactive: true
        padding: 0

        background: Rectangle {
            color: "#101620"
            border.color: root.line
        }

        ColumnLayout {
            anchors.fill: parent
            spacing: 0
            Rectangle {
                Layout.fillWidth: true
                Layout.preferredHeight: 66
                color: root.panel
                border.color: root.line
                Text {
                    anchors.left: parent.left
                    anchors.leftMargin: 20
                    anchors.verticalCenter: parent.verticalCenter
                    text: "DOWNLOADS"
                    color: root.ink
                    font.pixelSize: 16
                    font.weight: Font.Bold
                    font.letterSpacing: 1
                }
                Row {
                    anchors.right: parent.right
                    anchors.rightMargin: 12
                    anchors.verticalCenter: parent.verticalCenter
                    spacing: 5
                    RoundButton {
                        text: "↻"
                        flat: true
                        enabled: !downloadQueue.busy
                        onClicked: downloadQueue.refresh()
                    }
                    RoundButton {
                        text: "⌫"
                        flat: true
                        visible: downloadQueue.finished_count > 0
                        enabled: !downloadQueue.busy
                        onClicked: root.confirmClearDownloadHistory()
                        ToolTip.visible: hovered
                        ToolTip.text: "Clear finished history"
                    }
                    RoundButton {
                        text: "×"
                        flat: true
                        font.pixelSize: 20
                        onClicked: downloadsDrawer.close()
                    }
                }
            }
            Text {
                Layout.fillWidth: true
                Layout.margins: 16
                text: downloadQueue.message
                color: root.muted
                font.pixelSize: 11
                wrapMode: Text.WordWrap
            }
            RowLayout {
                Layout.fillWidth: true
                Layout.leftMargin: 16
                Layout.rightMargin: 16
                Layout.bottomMargin: 10
                visible: downloadQueue.job_count > 0
                spacing: 8
                Text {
                    text: downloadQueue.active_count + " active"
                    color: downloadQueue.active_count > 0 ? root.accentCool : root.muted
                    font.pixelSize: 11
                    font.weight: Font.DemiBold
                }
                Text {
                    text: "·"
                    color: root.muted
                }
                Text {
                    text: downloadQueue.aggregate_speed
                    color: root.ink
                    font.pixelSize: 11
                    font.features: { "tnum": 1 }
                }
                Item { Layout.fillWidth: true }
                Text {
                    text: downloadQueue.finished_count + " finished"
                    color: root.muted
                    font.pixelSize: 10
                }
            }
            ScrollView {
                Layout.fillWidth: true
                Layout.fillHeight: true
                clip: true
                ScrollBar.horizontal.policy: ScrollBar.AlwaysOff
                Column {
                    width: downloadsDrawer.width
                    spacing: 9
                    leftPadding: 14
                    rightPadding: 14
                    Repeater {
                        model: downloadQueue.job_count
                        delegate: Rectangle {
                            id: downloadRow
                            required property int index
                            property int queueRevision: downloadQueue.revision
                            width: downloadsDrawer.width - 28
                            height: 136
                            radius: 11
                            color: root.panelRaised
                            border.color: root.line
                            ColumnLayout {
                                anchors.fill: parent
                                anchors.margins: 12
                                spacing: 5
                                RowLayout {
                                    Layout.fillWidth: true
                                    Text {
                                        Layout.fillWidth: true
                                        text: { downloadRow.queueRevision; return downloadQueue.job_title_at(downloadRow.index) }
                                        color: root.ink
                                        font.pixelSize: 13
                                        font.weight: Font.DemiBold
                                        elide: Text.ElideRight
                                    }
                                    Text {
                                        text: { downloadRow.queueRevision; return downloadQueue.job_state_at(downloadRow.index) }
                                        color: text === "IMPORTED" ? root.accentCool : root.accent
                                        font.pixelSize: 9
                                        font.weight: Font.Bold
                                    }
                                }
                                Text {
                                    Layout.fillWidth: true
                                    text: { downloadRow.queueRevision; return downloadQueue.job_platform_at(downloadRow.index) }
                                    color: root.muted
                                    font.pixelSize: 10
                                    elide: Text.ElideRight
                                }
                                ProgressBar {
                                    Layout.fillWidth: true
                                    value: { downloadRow.queueRevision; return downloadQueue.job_progress_at(downloadRow.index) }
                                }
                                Text {
                                    Layout.fillWidth: true
                                    text: { downloadRow.queueRevision; return downloadQueue.job_detail_at(downloadRow.index) }
                                    color: root.muted
                                    font.pixelSize: 10
                                    elide: Text.ElideRight
                                }
                                RowLayout {
                                    Layout.fillWidth: true
                                    Item { Layout.fillWidth: true }
                                    Button {
                                        text: "Pause"
                                        visible: { downloadRow.queueRevision; return downloadQueue.job_can_pause(downloadRow.index) }
                                        enabled: !downloadQueue.busy
                                        onClicked: downloadQueue.pause_job(downloadRow.index)
                                    }
                                    Button {
                                        text: "Resume"
                                        visible: { downloadRow.queueRevision; return downloadQueue.job_can_resume(downloadRow.index) }
                                        enabled: !downloadQueue.busy
                                        onClicked: downloadQueue.resume_job(downloadRow.index)
                                    }
                                    Button {
                                        text: "Cancel"
                                        visible: { downloadRow.queueRevision; return downloadQueue.job_can_cancel(downloadRow.index) }
                                        enabled: !downloadQueue.busy
                                        onClicked: downloadQueue.cancel_job(downloadRow.index)
                                    }
                                    Button {
                                        text: "Remove"
                                        visible: { downloadRow.queueRevision; return downloadQueue.job_can_remove(downloadRow.index) }
                                        enabled: !downloadQueue.busy
                                        onClicked: root.confirmRemoveDownload(downloadRow.index)
                                        ToolTip.visible: hovered
                                        ToolTip.text: "Remove this history record; downloaded files are preserved"
                                    }
                                }
                            }
                        }
                    }
                    Item { width: 1; height: 14 }
                }
            }
        }
    }

    Rectangle {
        id: statusBar
        anchors.left: parent.left
        anchors.right: parent.right
        anchors.bottom: parent.bottom
        height: 34
        color: "#0a0f16"
        border.color: root.line
        Row {
            anchors.left: parent.left
            anchors.leftMargin: 15
            anchors.verticalCenter: parent.verticalCenter
            spacing: 9
            Rectangle {
                width: 7
                height: 7
                radius: 4
                color: library.loading || library.filtering || library.collection_busy ? root.accent :
                       library.ready ? root.accentCool : "#ef6a6a"
            }
            Text {
                text: library.collection_busy ? library.collection_message
                      : library.favorite_pending_count > 0
                        ? library.favorite_message : library.status_message
                color: root.muted
                font.pixelSize: 11
            }
        }
        Text {
            anchors.right: parent.right
            anchors.rightMargin: 15
            anchors.verticalCenter: parent.verticalCenter
            text: library.startup_ms > 0 ? "native shell " + library.startup_ms + " ms" : "native Qt"
            color: "#566175"
            font.pixelSize: 10
        }
    }
}
