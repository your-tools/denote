from datetime import datetime

import pytest

from denote import FrontMatter, Id, Metadata, slugify


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
    assert metadata.relative_path == "foo/bar/baz"


def test_can_parse_front_matter():
    text = """\
---
title: one
date: 2022-07-08 17:43:37
keywords: k1 k2
    """
    front_matter = FrontMatter.parse(text)

    assert front_matter.title == "one"
    assert front_matter.keywords == ["k1", "k2"]


def test_front_matter_roundtip():
    text = """\
---
title: Title
date: 2022-09-12 17:43:37
keywords: rust python
    """
    original = FrontMatter.parse(text)
    text = original.dump()
    loaded = FrontMatter.parse(text)

    assert loaded == original
