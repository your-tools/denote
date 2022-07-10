import shelve
import textwrap
from datetime import datetime

import pytest

from denote import (
    FrontMatter,
    Id,
    Metadata,
    Note,
    NotesRepository,
    slugify,
    get_note_from_markdown,
)


def test_slugify():
    assert slugify("This is a title") == "this-is-a-title"


def test_invalid_id():
    with pytest.raises(ValueError) as e:
        id = Id("bad")


def test_id_ordering():
    id1 = Id("20220707T142708")
    id2 = Id("20220707T142709")
    id3 = Id("20220707T142709")

    assert id1 < id2


def test_id_as_human_date():
    id = Id("20220707T142708")
    assert id.human_date() == "2022-07-07 14:27:08"


def test_can_build_id_from_date():
    now = datetime.now()
    id = Id.from_date(now)


def test_can_build_a_metadata_instance():
    id = Id("20220707T142708")
    metadata = Metadata(id, "This is a title", ["k1", "k2"], "md")

    assert metadata.id == str(id)
    assert metadata.title == "This is a title"
    assert metadata.keywords == ["k1", "k2"]
    assert metadata.extension == "md"
    assert metadata.relative_path == "2022/20220707T142708--this-is-a-title__k1_k2.md"


def test_can_parse_front_matter():
    text = textwrap.dedent(
        """\
        ---
        title: one
        date: 2022-07-08 17:43:37
        keywords: k1 k2
        """
    )
    front_matter = FrontMatter.parse(text)

    assert front_matter.title == "one"
    assert front_matter.keywords == ["k1", "k2"]


def test_front_matter_roundtip():
    text = textwrap.dedent(
        """\
        ---
        title: Title
        date: 2022-09-12 17:43:37
        keywords: rust python
        """
    )
    original = FrontMatter.parse(text)
    text = original.dump()
    loaded = FrontMatter.parse(text)

    assert loaded == original


def test_can_create_a_note():
    id = Id("20220707T142708")
    metadata = Metadata(id, "This is a title", ["k1", "k2"], "md")
    text = "this is my note\n"

    note = Note(text=text, metadata=metadata)

    front_matter = note.front_matter
    assert front_matter.title == "This is a title"
    assert front_matter.keywords == ["k1", "k2"]

    contents = note.dump()
    assert contents == textwrap.dedent(
        """\
        ---
        title: This is a title
        date: "2022-07-07 14:27:08"
        keywords: k1 k2
        ---
        this is my note
        """
    )


def test_cannot_open_a_repository_from_a_file():
    with pytest.raises(OSError):
        NotesRepository.open(__file__)


def test_markdown_import(tmp_path):
    notes_repository = NotesRepository.open(tmp_path)
    foo_md = tmp_path / "foo.md"
    contents = textwrap.dedent(
        f"""\
        ---
        title: This is a title
        date: {datetime.now}
        keywords: k1 k2
        ---
        this is my note
        """
    )
    foo_md.write_text(contents)
    saved_path = notes_repository.import_from_markdown(foo_md)

    new_text = (tmp_path / saved_path).read_text()
    actual_without_date = [
        x for x in new_text.splitlines() if not x.startswith("date: ")
    ]
    expected_without_date = [
        x for x in contents.splitlines() if not x.startswith("date: ")
    ]
    assert actual_without_date == expected_without_date


def test_loading_and_saving(tmp_path):
    id = Id("20220707T142708")
    metadata = Metadata(id, "This is a title", ["k1", "k2"], "md")
    text = "this is my note\n"

    note = Note(text=text, metadata=metadata)

    notes_repository = NotesRepository.open(tmp_path)
    relative_path = notes_repository.save(note)

    notes_repository.load(relative_path)


def test_update_note_path_when_title_changes(tmp_path):
    id = Id("20220707T142708")
    metadata = Metadata(id, "old title", ["k1", "k2"], "md")
    text = "this is my note\n"

    note = Note(text=text, metadata=metadata)
    assert "--old-title" in note.relative_path

    notes_repository = NotesRepository.open(tmp_path)
    relative_path = notes_repository.save(note)

    contents = (tmp_path / relative_path).read_text()
    (tmp_path / relative_path).write_text(contents.replace("old title", "new title"))

    note = notes_repository.load(relative_path)
    notes_repository.save(note)
    assert "--new-title" in note.relative_path


def test_update_note_path_when_keywords_change(tmp_path):
    id = Id("20220707T142708")
    metadata = Metadata(id, "title", ["k1", "k2"], "md")
    text = "this is my note\n"

    note = Note(text=text, metadata=metadata)
    assert "__k1_k2" in note.relative_path

    notes_repository = NotesRepository.open(tmp_path)
    relative_path = notes_repository.save(note)

    contents = (tmp_path / relative_path).read_text()
    (tmp_path / relative_path).write_text(contents.replace("k1 k2", "tag1 tag2"))

    note = notes_repository.load(relative_path)
    notes_repository.save(note)
    assert "__tag1_tag2" in note.relative_path


class NoteShelf:
    def __init__(self, shelve_path):
        self.shelve_path = shelve_path

    def __enter__(self):
        self.db = shelve.open(self.shelve_path)
        return self

    def save(self, note):
        self.db[note.id] = note.dump()

    def load(self, id):
        markdown = self.db[str(id)]
        return get_note_from_markdown(id, markdown)

    def notes(self):
        for id_s, markdown in self.db.items():
            id = Id(id_s)
            yield get_note_from_markdown(id, markdown)

    def __exit__(self, *args):
        self.db.close()


def test_add_note_to_empty_shelf(tmp_path):
    id = Id("20220707T142708")
    metadata = Metadata(id, "title", ["k1", "k2"], "md")
    text = "this is my note\n"
    note = Note(text=text, metadata=metadata)

    shelve_path = str(tmp_path / "notes.shelve")

    with NoteShelf(shelve_path) as shelf:
        shelf.save(note)

    with NoteShelf(shelve_path) as shelf:
        saved = shelf.load(id)

    assert saved == note


def test_update_title_of_note_in_shelf(tmp_path):
    id = Id("20220707T142708")
    metadata = Metadata(id, "old title", ["k1", "k2"], "md")
    text = "this is my note\n"
    note = Note(text=text, metadata=metadata)

    shelve_path = str(tmp_path / "notes.shelve")

    with NoteShelf(shelve_path) as shelf:
        shelf.save(note)

    with NoteShelf(shelve_path) as shelf:
        note = shelf.load(id)
        contents = note.dump()
        new_contents = contents.replace("old title", "new title")
        new_note = get_note_from_markdown(id, new_contents)
        shelf.save(new_note)

        actual = list(shelf.notes())
        assert len(actual) == 1


def test_add_two_notes_in_shelf(tmp_path):
    shelve_path = str(tmp_path / "notes.shelve")

    first_id = Id("20220707T142708")
    first_metadata = Metadata(first_id, "First!", ["one"], "md")
    first_text = "I am the first note - so happy !"
    first_note = Note(text=first_text, metadata=first_metadata)

    second_id = Id("20220708T152912")
    second_metadata = Metadata(second_id, "Second :(", ["two"], "md")
    second_text = "I am the second note - so sad :("
    second_note = Note(text=second_text, metadata=second_metadata)

    with NoteShelf(shelve_path) as shelf:
        shelf.save(first_note)
        shelf.save(second_note)

    with NoteShelf(shelve_path) as shelf:
        saved = list(shelf.notes())

    assert sorted(saved) == [first_note, second_note]
