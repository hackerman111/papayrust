use std::path::PathBuf;
use uuid::Uuid;

/// Actions supported by the Papyrus application and user interface.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Action {
    NextPanel,
    PreviousPanel,
    MoveUp,
    MoveDown,
    Open,
    OpenAtPage(u32),
    Search,
    SearchInput(char),
    SearchBackspace,
    SearchConfirm,
    SearchCancel,
    ExportLibrary,
    EditMetadata,
    EditMetadataNextField,
    EditMetadataPrevField,
    EditMetadataInput(char),
    EditMetadataBackspace,
    EditMetadataSave,
    EditMetadataCancel,
    OpenToc,
    TocAddEntry { parent_id: Option<Uuid> },
    TocEditEntry { id: Uuid },
    TocDeleteEntry { id: Uuid },
    TocIndent { id: Uuid },
    TocOutdent { id: Uuid },
    TocMoveUp { id: Uuid },
    TocMoveDown { id: Uuid },
    TocEmbed,
    TocImport { source: TocImportSource },
    TocEditModalNextField,
    TocEditModalPrevField,
    TocEditModalInput(char),
    TocEditModalBackspace,
    TocEditModalSave,
    TocEditModalCancel,
    TocImportModalOpen,
    TocImportModalNextField,
    TocImportModalPrevField,
    TocImportModalInput(char),
    TocImportModalBackspace,
    TocImportModalToggleSource,
    TocImportModalToggleMerge,
    TocImportModalConfirm,
    TocImportModalCancel,
    AddPaperModalOpen,
    AddPaperModalInput(char),
    AddPaperModalBackspace,
    AddPaperModalConfirm,
    AddPaperModalCancel,
    CreateCollectionModalOpen,
    CreateCollectionModalInput(char),
    CreateCollectionModalBackspace,
    CreateCollectionModalConfirm,
    CreateCollectionModalCancel,
    OpenFullscreenToc,
    CloseFullscreenToc,
    EditTagsModalOpen,
    EditTagsModalInput(char),
    EditTagsModalBackspace,
    EditTagsModalConfirm,
    EditTagsModalCancel,
    DeleteConfirmOpen,
    DeleteConfirmExecute,
    DeleteConfirmCancel,
    SearchCycleCollection,
    HelpModalToggle,
    CreateSubcollectionModalOpen,
    RenameCollectionModalOpen,
    RenameCollectionModalInput(char),
    RenameCollectionModalBackspace,
    RenameCollectionModalConfirm,
    RenameCollectionModalCancel,
    ExportCollectionModalOpen,
    ExportCollectionModalInput(char),
    ExportCollectionModalBackspace,
    ExportCollectionModalConfirm,
    ExportCollectionModalCancel,
    ExportCollectionModalAutocomplete,
    ImportMetadataModalOpen,
    ImportMetadataModalInput(char),
    ImportMetadataModalBackspace,
    ImportMetadataModalConfirm,
    ImportMetadataModalCancel,
    ImportMetadataModalAutocomplete,
    AddPaperModalAutocomplete,
    Quit,
}

/// Source for importing table of contents entries into a paper.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TocImportSource {
    PdfOutline,
    TextFile(PathBuf),
    JsonFile(PathBuf),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_action_variants_equality() {
        assert_eq!(Action::NextPanel, Action::NextPanel);
        assert_ne!(Action::NextPanel, Action::PreviousPanel);

        let id = Uuid::nil();
        assert_eq!(
            Action::TocAddEntry {
                parent_id: Some(id)
            },
            Action::TocAddEntry {
                parent_id: Some(id)
            }
        );
        assert_ne!(
            Action::TocAddEntry {
                parent_id: Some(id)
            },
            Action::TocAddEntry { parent_id: None }
        );

        assert_eq!(Action::Search, Action::Search);
        assert_eq!(Action::SearchInput('a'), Action::SearchInput('a'));
        assert_ne!(Action::SearchInput('a'), Action::SearchInput('b'));
        assert_eq!(Action::SearchBackspace, Action::SearchBackspace);
        assert_eq!(Action::SearchConfirm, Action::SearchConfirm);
        assert_eq!(Action::SearchCancel, Action::SearchCancel);
        assert_ne!(Action::SearchConfirm, Action::SearchCancel);

        assert_eq!(Action::EditMetadata, Action::EditMetadata);
        assert_eq!(Action::EditMetadataNextField, Action::EditMetadataNextField);
        assert_eq!(Action::EditMetadataPrevField, Action::EditMetadataPrevField);
        assert_eq!(
            Action::EditMetadataInput('x'),
            Action::EditMetadataInput('x')
        );
        assert_ne!(
            Action::EditMetadataInput('x'),
            Action::EditMetadataInput('y')
        );
        assert_eq!(Action::EditMetadataBackspace, Action::EditMetadataBackspace);
        assert_eq!(Action::EditMetadataSave, Action::EditMetadataSave);
        assert_eq!(Action::EditMetadataCancel, Action::EditMetadataCancel);

        let src = TocImportSource::PdfOutline;
        assert_eq!(
            Action::TocImport {
                source: src.clone()
            },
            Action::TocImport { source: src }
        );
        assert_eq!(
            TocImportSource::TextFile(PathBuf::from("toc.txt")),
            TocImportSource::TextFile(PathBuf::from("toc.txt"))
        );

        assert_eq!(Action::TocEditModalSave, Action::TocEditModalSave);
        assert_eq!(Action::TocEditModalCancel, Action::TocEditModalCancel);
        assert_eq!(Action::TocEditModalNextField, Action::TocEditModalNextField);
        assert_eq!(Action::TocEditModalPrevField, Action::TocEditModalPrevField);
        assert_eq!(
            Action::TocEditModalInput('a'),
            Action::TocEditModalInput('a')
        );
        assert_eq!(Action::TocEditModalBackspace, Action::TocEditModalBackspace);
        assert_eq!(Action::TocImportModalOpen, Action::TocImportModalOpen);
        assert_eq!(
            Action::TocImportModalNextField,
            Action::TocImportModalNextField
        );
        assert_eq!(
            Action::TocImportModalPrevField,
            Action::TocImportModalPrevField
        );
        assert_eq!(Action::TocImportModalConfirm, Action::TocImportModalConfirm);
        assert_eq!(Action::TocImportModalCancel, Action::TocImportModalCancel);
        assert_eq!(
            Action::TocImportModalToggleSource,
            Action::TocImportModalToggleSource
        );
        assert_eq!(
            Action::TocImportModalToggleMerge,
            Action::TocImportModalToggleMerge
        );
        assert_eq!(
            Action::TocImportModalInput('z'),
            Action::TocImportModalInput('z')
        );
        assert_eq!(
            Action::TocImportModalBackspace,
            Action::TocImportModalBackspace
        );
    }
}
