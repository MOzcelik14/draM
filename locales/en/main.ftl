# draM. English UI Translations

app-title = draM. — Screenplay Editor
untitled-document = Untitled Screenplay

menu-file = File
menu-new = New
menu-open = Open...
menu-save = Save
menu-save-as = Save As...
menu-import-fountain = Import Fountain...
menu-export-fountain = Export Fountain...
menu-export-pdf = Export PDF...
menu-title-page = Title Page...
menu-edit = Edit
menu-undo = Undo
menu-redo = Redo
menu-settings = Settings

ui-language-label = UI Language:
ui-language-tr = Türkçe
ui-language-en = English

script-language-label = Script Language:
script-language-tr = Turkish (İÇ. / DIŞ.)
script-language-en = English (INT. / EXT.)

page-size-label = Page Size:
page-size-a4 = A4 (210 × 297 mm)
page-size-us-letter = US Letter (8.5 × 11 in)
show-scene-numbers = Show Scene Numbers

title-page-heading = Title Page
tp-title = Title
tp-credit = Credit
tp-author = Author
tp-source = Source
tp-draft-date = Draft Date
tp-contact = Contact
tp-copyright = Copyright
btn-close = Close

toolbar-element-type = Element Type:
elem-scene-heading = Scene Heading
elem-action = Action
elem-character = Character
elem-parenthetical = Parenthetical
elem-dialogue = Dialogue
elem-transition = Transition
elem-shot = Shot
elem-general = General
elem-centered = Centered
elem-lyrics = Lyrics
elem-page-break = Page Break
elem-section = Section
elem-synopsis = Synopsis
elem-note = Note

status-scenes-count = { $count ->
    [one] { $count } scene
   *[other] { $count } scenes
}
status-elements-count = { $count ->
    [one] { $count } line
   *[other] { $count } lines
}
status-pages-runtime = { $pages ->
    [one] { $pages } page (~{ $minutes } min)
   *[other] { $pages } pages (~{ $minutes } min)
}
status-saved = Saved
status-modified = Modified
status-hint-flow = Enter: Next element | Tab: Cycle type | Ctrl+1..8: Quick type | Ctrl+S: Save

error-open-failed = Could not open file: { $reason }
error-save-failed = Could not save file: { $reason }
error-pdf-failed = Could not export PDF: { $reason }

menu-import-fadein = Import Fade In...
menu-export-fadein = Export Fade In...
menu-print = Print...
menu-find-replace = Find / Replace...

nav-heading = Scene Navigator
nav-toggle = Navigator
nav-move-up = Up
nav-move-down = Down

print-dialog-title = Print
print-printer = Printer:
print-default-printer = System Default
print-copies = Copies:
print-page-range = Page Range (e.g. 1-5):
print-btn = Print
print-open-viewer = Open PDF in Default Viewer
print-success = Print job submitted.

find-label = Find:
replace-label = Replace:
find-case-sensitive = Case Sensitive
find-next-btn = Find Next
replace-btn = Replace
replace-all-btn = Replace All
find-no-match = No match found.

autocomplete-suggestions = Suggestions:

menu-import-fdx = Import Final Draft (.fdx)...
menu-export-fdx = Export Final Draft (.fdx)...

theme-label = Theme:
theme-dark = Dark
theme-light = Light
focus-mode-toggle = Focus Mode (F11)

notes-heading = Notes
notes-toggle = Notes
notes-add = + Add Note
notes-delete = Delete
notes-title-placeholder = Note title
notes-body-placeholder = Note body...

dual-dialogue-btn = Dual Dialogue
dual-left-badge = [DUAL LEFT]
dual-right-badge = [DUAL RIGHT]

revision-label = Revision:
rev-none = None
rev-blue = Blue
rev-pink = Pink
rev-yellow = Yellow
rev-green = Green

tab-format = Format
tab-characters = Characters
tab-locations = Locations
tab-notes = Notes
page-x-of-y = Page { $current } of { $total }
empty-list = No entries yet.

menu-view = View
menu-format = Format
menu-document = Document
menu-help = Help
menu-import = Import
menu-export = Export
menu-renumber-scenes = Renumber Scenes
menu-shortcuts = Keyboard Shortcuts...
shortcuts-title = Keyboard Shortcuts
shortcuts-body = Enter: Insert next element type
    Tab: Cycle current element type
    Ctrl+1..8: Select element type directly
    Ctrl+S: Save project
    Ctrl+Z / Ctrl+Y: Undo / Redo
    Ctrl+F: Find / Replace
    Ctrl+P: Print
    F11: Focus Mode

menu-about = About draM....
about-title = About draM.
about-version = Version { $version }
about-desc = Desktop screenplay writing application built for Turkish and English scripts.
about-suite = Part of the prisM. Suite.
about-license = License: MIT / Apache-2.0 | Font: Courier Prime (SIL OFL 1.1)

unsaved-title = Unsaved Changes
unsaved-message = You have unsaved changes. Do you want to save before closing?
btn-save-and-close = Save
btn-discard-close = Close Without Saving
btn-cancel = Cancel

recovery-title = Recovery File Found
recovery-message = An unsaved session from an unexpected exit was found. Would you like to recover it?
btn-recover = Recover
btn-discard-recovery = Discard

fmt-bold = Bold (Ctrl+B)
fmt-italic = Italic (Ctrl+I)
fmt-underline = Underline (Ctrl+U)

view-editor = Editor
view-beat-board = Index Cards
beat-board-title = Index Cards
beat-board-empty = No scene headings yet. Add a scene heading in the editor to get started.
beat-board-open = Open in Editor
beat-board-synopsis-placeholder = Add scene synopsis...
beat-board-move-prev = Move Earlier
beat-board-move-next = Move Later

rev-mode-toggle = Revision Mode
rev-mode-active = Revision: On
rev-clear-all = Clear Revisions

menu-reports = Script Reports...
reports-title = Script Reports
reports-summary = Total { $scenes } scenes · INT.: { $int } · EXT.: { $ext } · DAY: { $day } · NIGHT: { $night }
reports-tab-characters = Characters
reports-tab-locations = Locations
reports-col-name = Character
reports-col-lines = Blocks
reports-col-words = Words
reports-col-scenes = Scenes
reports-col-location = Location
reports-col-int-ext = INT./EXT.
reports-col-day-night = DAY/NIGHT
reports-export-csv = Export CSV...
reports-exported = Report exported to CSV.

menu-new-template = New from Template
template-feature = Feature Film
template-short = Short Film
template-series = TV Episode

menu-recent-files = Recent Files
menu-recent-empty = (No recent files)
menu-recent-clear = Clear List

tp-preview-heading = Live Preview

spellcheck-toggle = Spellcheck
spell-add-dict = Add to Dictionary
spell-no-suggestions = (No suggestions)

notes-attach-image = Attach Image...
notes-remove-image = Remove Image
notes-preview-title = Reference Image
