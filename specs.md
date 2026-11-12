# Herb's Contacts Optimizer: specification

A desktop app that cleans up address books: it finds duplicates, merges and repairs contacts, and moves them
between files and accounts. For macOS, Linux and Windows, written in Rust. The name is tentative.

What is built so far is tracked in `pending.md`; this file describes the whole app.

## 1. Reference app

The feature list comes from looking closely at Contacts Optimizer 6.1.400 for Android (Compelson Labs): what
it offers, what its screens say and how its steps behave. This app does the same jobs where that makes
sense, but it is written from scratch. None of that app's code, text, data or artwork is in here.

### 1.1 What the reference app does

#### Start screen

| Function | What it does |
|---|---|
| Select account | Lists the accounts on the phone with their contact counts, empty and read-only ones included. One account is optimized at a time. Tap and hold shows its contacts. |
| Duplicates counter | Shows how many duplicates were found; a tap opens them, a second search looks for similar contacts. |
| Delete duplicates | Removes all exact duplicates in one step, after a confirmation. |
| Optimization wizard | Runs the chosen steps one after another (see below). |
| Toolbox | Opens any single step on its own. |
| View contacts / Edit contacts | The contact list; read-only accounts can only be viewed. |
| Sync warning | Warns when contact sync is on and offers the sync settings. |

#### Wizard and Toolbox steps

| Group | Step | What it does |
|---|---|---|
| Duplicates and merging | Duplicates | Groups identical contacts; duplicate fields are marked red and deleted when ticked. |
| | Inner duplicates | Removes repeated values inside one contact. |
| | Similar contacts | Finds contacts that are probably the same person; the user ticks which to delete. |
| | Merging contacts | Proposes merging simple contacts into one rich contact; shows what is added to the primary one. |
| | Manual merging | The user picks two or more contacts, orders them (move up, move down), previews and merges. |
| Advanced optimization | Invalid contacts | Lists contacts that look wrong, for deletion. |
| | Missing information | Lists incomplete contacts; a tap opens the editor. |
| | Fill in empty names | Builds a missing display name from e-mail, organization or name parts. |
| | Capitalization | Fixes letter case in names. |
| | Internationalization | Adds the country code to phone numbers. The country is chosen from a list; country code, international prefix and trunk prefix can be changed. |
| | Simplify numbers | Strips non-digit characters from phone numbers. |
| | Search and replace | Replaces a text in all contact data, every occurrence. |
| | Mass delete of contacts | Deletes the ticked contacts. |
| | Mass delete of data | Deletes one type of data (for example all notes) from the ticked contacts. |
| | Unresolved numbers | Offers numbers from call logs and messages that are not saved yet as new contacts. |
| Accounts | Mass copy between accounts | Copies the ticked contacts to another account, also from read-only accounts. |
| | Mass move between accounts | Moves the ticked contacts to another account. |
| | Move all contacts | Moves everything from the chosen accounts into one. |
| Finish | Final preview | Shows all contacts with the changed ones in red; a tap edits. |
| | Save changes | Writes everything at once, after a confirmation. Nothing is written before this. |
| | Summary | Counts per step: found, deleted, merged, repaired, added; contacts before, after and modified. |

#### In every step

- A description of the step, a "Step N of M" and "Page N of M" header, and the count of selected contacts.
- Proposed changes are a tick list; obvious ones are ticked already. Original and suggested data are shown side by side.
- Select all, deselect all, sort, filter, preview.
- Skip the current entries, skip the whole step, or finish the step.
- Tap and hold a contact: view the complete contact, edit, delete, copy to account, move to account.
- Quitting asks whether to save or drop the changes.

#### Contact list

- Sort by a chosen data type.
- Filter by text (with or without it), by data (has or lacks a field), by group (in or outside it).
- Add contact.

#### Contact editor

- Name prefix, first, middle and last name, name suffix, nickname, display name.
- Photo: add, change, remove.
- Phones, e-mails, addresses (street, PO box, neighborhood, city, state, ZIP code, country), organizations
  (company, department, title, position, address) and websites, each with a type and as many rows as needed.
- Birthday, anniversary, note, groups, instant-messaging names, relatives.

#### Other

- About page with version, what's new and a short guide.
- Logging to a chosen folder; an error report after a crash.
- A paid PRO version unlocks saving the advanced steps; share and rate links; links to the maker's other apps.

## 2. What our app does

Everything in 1.1 except the paid version, the share and rate links, the advertising links, and the
unresolved-numbers step (a desktop has no call log). On top of that:

- It works on **files** and **accounts** alike. Each opened file and each connected account is an *address
  book* in the sidebar.
- It reads and writes every common contact format (section 4).
- It connects to Google, Microsoft (Outlook) and iCloud accounts, and to any CardDAV server (section 5).
- Nothing is changed until the user saves. Before a save to a file or an account, the app keeps a backup of
  what was there, and Restore brings it back.

## 3. Window

One window. From left to right: the sidebar with address books, the contact list, and the selected contact.

- **Main Menu** (☰), the first button of the toolbar: File, Edit, View, Tools, Accounts, Help. Every command
  is there; the macOS menu bar keeps only the app menu.
- **Toolbar**: Main Menu, Open, Save, Export, Find Duplicates, Wizard, Toolbox, New Contact, and a search field.
- **Sidebar**: address books with contact counts; read-only ones are marked.
- **Contact list**: name and main phone or e-mail, tick boxes for mass actions, sort and filter.
- **Details**: the full contact; Edit switches to the editor (1.1, Contact editor).
- **Status bar**: contacts shown, selected, and unsaved changes.
- Every control has a hover text that says what it does and its shortcut, and a label for screen readers.

| Command | Shortcut (macOS; Ctrl on Linux and Windows) |
|---|---|
| Open file | ⌘O |
| Save | ⌘S |
| Export | ⇧⌘E |
| New contact | ⌘N |
| Find | ⌘F |
| Restore backup | Main Menu → File → Restore Backup |
| Optimization wizard | Main Menu → Tools → Optimization Wizard |
| Find duplicates | ⌘D |
| Toolbox | Main Menu → Tools → Toolbox |
| Edit contact | Main Menu → Edit → Edit Contact |
| Select all | ⌘A |
| Delete contact | ⌫, asks first, Cancel is the default |
| About | Main Menu → Help → About |

### 3.2 Contact list

Each row has a tick box left of the name; the box in the header ticks or unticks every contact shown, and so
do Select All (⌘A) and Deselect All (Esc). The status bar counts the ticked ones. A click on Name, Phone or
E-mail in the header orders the list by that column, a second click turns the order around; View → Sort By
also offers Last Name and the file's own order. Contacts without the value sorted by come last.

| Key | In the list |
|---|---|
| ↑ ↓ | the contact above or below |
| Home, End | the first or last contact |
| Return | edit the contact |
| Space | tick or untick it |
| ⌫ | delete it, after asking |
| ⌘A, Esc | tick all shown, untick all |

The keys rest while a dialog is open or a text field has the cursor.

Export writes the ticked contacts, or all of them when none is ticked.

Edit → Edit Ticked Contacts changes all ticked contacts at once: company, department, title, city, state,
country, add to a group, remove from a group, and a line added to the note. Only the fields filled in are
applied. Delete (⌫) is about the ticked contacts when there are any, else about the selected one; it asks
first, with Cancel as the default.

### 3.3 Several address books

Several files can be opened at once, in the Open dialog or by dropping them on the window. In the sidebar a
click selects one address book, ⌘-click adds or removes one, Shift-click takes the range, a drag sweeps over
several, and ⌘A takes all when the sidebar was clicked last (after a click in the list it ticks contacts).
With two or more selected, Merge Address Books (sidebar button, or Main Menu → File) puts their contacts
into one new address book, where every tool works on them together; Export keeps it as a file. Contacts
copied from an account belong to no account in the merged address book. Close Address Book closes the
selected ones.

### 3.4 Appearance

View → Appearance offers As the System, Light and Dark. The app remembers the choice in its own folder
(`settings.txt`) and starts with it.

Both appearances use the colours of macOS 27's Finder: its window, sidebar and bar greys, its striped rows,
its blue selection with white text, its separators and its text colours. Only the colours are Finder's;
the layout and the shapes are the app's own: the toolbar across the top with the Main Menu at its left, the
address books below it at the left, rounded-rectangle buttons, and the status bar along the bottom.

### 3.1 About

Help → About Herb's Contacts Optimizer (and the app menu on macOS) shows the icon, the name, the version, the
line "Clean up your contacts.", "By dunkelherzmann" and the project link. The version comes from the `VERSION`
file plus the number of commits since it last changed.

## 4. Formats

| Format | Extensions | Read | Write |
|---|---|---|---|
| vCard 2.1, 3.0, 4.0 (single and multi-contact, folded lines, quoted-printable, embedded pictures) | `.vcf`, `.vcard` | yes | 3.0 and 4.0 |
| CSV: Google, Outlook, Thunderbird and others by their column names | `.csv`, `.tsv`, `.txt` | yes | Google and Outlook layouts (Outlook's columns follow Microsoft's import template) |
| Excel workbook: the first sheet, with a header row | `.xlsx` | yes | yes, in Google's columns, every cell as text |
| LDIF (Thunderbird, LDAP directories), with the attribute names Thunderbird's importer accepts | `.ldif`, `.ldi` | yes | yes |
| jCard (RFC 7095) | `.json` | yes | yes |
| xCard (RFC 6351) | `.xml` | yes | yes |

Each of these can be opened, saved back in place and chosen in File → Export. LDIF and Outlook CSV have a
fixed number of places (two or three e-mail addresses, one phone number of each kind, a work and a home
address); what does not fit is left out when writing them. Pictures travel in vCard, jCard and xCard.

Not built yet:

| Format | Extensions | Planned |
|---|---|---|
| Any CSV layout through column mapping | `.csv` | read |
| Windows Contacts | `.contact` | read and write |
| Outlook items and stores | `.msg`, `.pst`, `.ost` | read |
| Outlook Express / Windows Address Book | `.wab` | read |
| macOS Contacts archive | `.abbu` | read |
| Thunderbird address book | `.mab`, `abook.sqlite` | read |
| Android contacts database | `contacts2.db` | read |
| Nokia and Samsung backups | `.nbu`, `.nbf`, `.spb` | read |

The format is recognised from the content, not the extension. Text encodings are detected (UTF-8, UTF-16,
Latin-1).
Unknown vCard properties are kept and written back unchanged.

## 5. Accounts

| Account | How it connects | Sign-in |
|---|---|---|
| Google | People API | Browser sign-in (OAuth) |
| Microsoft: Outlook.com, Microsoft 365 | Microsoft Graph | Browser sign-in (OAuth) |
| iCloud | CardDAV | Apple ID and an app-specific password |
| Any CardDAV server (Fastmail, Nextcloud, …) | CardDAV | Address, user name, password |

For each account the app can scan, view, edit, add and delete contacts and groups, import files into it,
export it to any format in section 4, copy or move contacts between accounts and files, and run every
optimization step on it. Changes go to the account only on Save.

Accounts → Add Account asks for iCloud (Apple Account and an app-specific password) or another server
(address, user name, password). With the first account the user chooses the app password. Each address book
of the account appears in the sidebar under the account's name. Save on such an address book asks first,
saying how many contacts are changed, new and deleted, with Cancel as the default; the contacts as the account
had them are written to the backups folder before anything is sent. A contact changed on the server in the
meantime is not overwritten. At the next start the sidebar offers Unlock accounts. Load Again reads the
account anew; Remove Account takes it out of the app and its store and leaves the server alone.

Sign-in tokens and passwords are kept in the app's own encrypted store, protected by an app password the user
sets. The macOS Keychain is not used. Nothing is sent anywhere except to the account's own service.

### 5.1 The computer's own accounts

Besides a CardDAV address and password, the app can read the accounts already signed in to the operating
system (iCloud, Google, Exchange and the rest, as set up in the system's own settings), through the OS, so
it needs no account of its own and no developer registration. This is how the Android reference app reached
online accounts: it read the phone's system contacts, where the OS already held them.

One backend per platform, each behind the same seam as a CardDAV account (`core/src/account.rs`,
`ContactStore`):

| Platform | Through | Read | Write back |
|---|---|---|---|
| macOS | the Contacts framework (`CNContactStore`) | every configured account | yes |
| Linux | Evolution Data Server with GNOME Online Accounts (D-Bus) | where that stack is installed | yes, there |
| Windows | the WinRT contact store | yes | limited by the platform; may be read-only |

The OS asks for permission the first time (on the Mac, the system's Contacts prompt). An account read this way
appears in the sidebar like any other, and its changes go back only on Save. None of the backends is built
yet; `core/src/osstore.rs` holds the seam and says so until one is.

## 6. Optimization rules

| Step | Rule |
|---|---|
| Comparing | Text is compared without case, accents, spacing and punctuation; e-mail addresses without case. Two phone numbers match when their digits are equal, or when one is the other with a country code or a leading 0 in front. |
| Duplicates | A contact is a duplicate of another when it has the same name and everything it holds is also in the other one. The fuller contact is kept; of two equal ones, the first. |
| Inner duplicates | Inside one contact, values that match are reduced to one; of a number with and without its country code, the longer form stays. |
| Similar contacts | The same name in any word order; the same last name with one first name an initial or a short form of the other ("J. Smith", "John Smith"); a shared phone number or e-mail address; or names that differ by a typo. Shown as sets; nothing is ticked in advance. |
| Merging | The primary contact keeps its fields; fields the others have and it lacks are added. |
| Invalid contacts | No name and no data; a name of one character that is not an initial, or with `! ? _ # $ % * < >` in it; a phone number that is not 112 or 911, does not start with `*`, and has fewer than 5 or more than 15 digits; an e-mail address without exactly one `@`, or without a domain with a dot and two letters after it. |
| Missing information | No name, or no phone number and no e-mail address. |
| Fill in empty names | The nickname, then the organization, then the part of the e-mail address before `@`, then the phone number. |
| Capitalization | "JOHN SMITH" and "john smith" become "John Smith" and "MCDONALD" becomes "McDonald"; particles (van, de, von), names already in mixed case and e-mail addresses are left alone. |
| Search and replace | Replaces the exact text, upper and lower case included, everywhere in the ticked contacts. |
| Country codes | With the chosen country: an international prefix becomes `+`, a trunk prefix is dropped, and the country code is put in front of national numbers. |
| Simplify numbers | Everything but digits and a leading `+` is removed. |

### 6.1 Toolbox and editor

Toolbox (toolbar, or Main Menu → Tools → Toolbox) lists every step on its own, with a line saying what it
does. A step opens as a tick list of what it would change, with the count selected, Select All and Deselect
All. Repairs and exact duplicates come ticked; anything else that deletes or merges comes unticked, and a
button that deletes is red. Nothing changes until the step's button is pressed, and the file changes only on
Save.

The editor (Edit in the contact's panel, Main Menu → Edit → Edit Contact, or New Contact, ⌘N) is a form over a
copy of the contact: name parts, phones, e-mails, addresses, organization, websites, birthday, anniversary,
groups and note, and the picture: Add or Change Picture takes a JPEG, PNG or GIF file and keeps it byte for
byte; Remove Picture takes it off. The contact's panel shows the picture above the name. Pictures are read
from and written to vCard files of every version; a picture that is only a web address stays as it is.
Save Contact keeps the changes; Cancel or Esc drops them. Delete Contact (⌫) asks first, with Cancel
as the default.

Every line of a step has Edit and Delete for its contact. Edit opens the editor and comes back to the step;
Delete asks first, with Cancel as the default, and comes back too. The step then works out its lines anew.
In Merge Contacts each set has an "Into" dropdown to choose which contact keeps its data and gains the
others'; in Merge by Hand "Merge into" does the same for the ticked contacts. One at a Time shows a single
line with the contact's main details and Previous, Leave (untick and go on) and Select (tick and go on);
after the last line the list is back for a look and the step's button.

Country Codes lists every phone number without a country code, one line per number, with the number as it
would become. "Country for all" sets the country, from a dropdown of countries with their codes or by typing
the code; each line has the same two controls for a number from elsewhere. A line changed by hand keeps its
choice when the country for all changes. The country's international prefix (00, 011…) becomes the plus and
its trunk prefix (0) is dropped. Emergency and service numbers are left out. In the editor each phone number
has a code dropdown that shows its country code and puts the chosen country's code in front; a note appears
while a number has none. Country data comes from libphonenumber and Unicode CLDR (`THIRD-PARTY.md`).

### 6.2 Optimization wizard

Wizard (toolbar, or Main Menu → Tools → Optimization Wizard) first lists its steps, all ticked: Duplicates,
Inner Duplicates, Similar Contacts, Merge Contacts, Invalid Contacts, Missing Information, Fill In Empty
Names, Capitalization, Country Codes, Simplify Numbers. Start runs the ticked ones in that order. Each is the same tick list
as in the Toolbox, with "Step N of M" above it, its own button to carry it out, and Skip. A step that finds
nothing is passed over. Cancel Wizard stops and puts every contact back as it was. After the last step the
Summary shows the contacts before and now and what each step did; Undo All puts everything back, Done keeps
the changes.

### 6.3 Saving

Save (⌘S) writes the address book back to its file in the file's own format: vCard 4.0 stays 4.0, vCard 2.1
and 3.0 are written as 3.0, and Google CSV, Outlook CSV, Excel, LDIF, jCard and xCard each stay what they are. Before writing, the file as it was is copied to the
app's backups folder; the ten newest backups of each file are kept. A CSV file in another layout is not
overwritten: the app says so and points to Export. An address book without a file goes to Export.

File → Restore Backup puts the newest backup back over the file, after asking with Cancel as the default, and
shows its contacts again. Restoring again reaches the backup before that.

Closing the window with unsaved changes asks first: Cancel, the default, or Quit Without Saving.

## 7. Data and safety

- Settings, backups and logs are in the app's own folder (`~/Library/Application Support/HerbsContactsOptimizer`
  on macOS, `%APPDATA%\HerbsContactsOptimizer` on Windows, the Flatpak data folder on Linux).
- Deleting and overwriting ask first, with Cancel as the default.
- Logging is off unless switched on in Help → Logging.

## 8. Platforms

| Platform | Package |
|---|---|
| macOS | `Herb's Contacts Optimizer.app`, ad-hoc signed |
| Linux | Flatpak `org.herbscontactsoptimizer.app` |
| Windows | one portable `.exe` |

The three versions look and work the same; the core (`core/`) and the window (`app/`, egui) are shared Rust,
and each is built and tested on its own system. What differs:

- Each system draws its own title bar, with its window buttons where that system puts them. Below it the
  toolbar is the same on all three: Main Menu first, then Open, Save, Export, Find Duplicates, Wizard,
  Toolbox and New Contact, in that order and in the same places. Shortcuts use Ctrl where the Mac uses ⌘.
- The Mac draws text in its own font; Linux and Windows carry the Inter font inside the app.
- `--self-check` starts the program without a window, tries the core, and says whether it passed; every
  build script ends with it. On Windows it prints nothing; its exit code tells.
