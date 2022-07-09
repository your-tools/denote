import textwrap
from datetime import datetime

import pytest

from denote import FrontMatter, Id, Metadata, Note, NotesRepository, slugify


def test_slugify():
    assert slugify("This is a title") == "this-is-a-title"


def test_invalid_id():
    with pytest.raises(ValueError) as e:
        id = Id("bad")


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
