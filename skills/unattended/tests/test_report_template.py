import unittest
from html.parser import HTMLParser
from pathlib import Path

REPORT = Path(__file__).parents[1] / "templates/REPORT.html"


class StructureParser(HTMLParser):
    def __init__(self):
        super().__init__()
        self.ids = set()
        self.tags = []

    def handle_starttag(self, tag, attrs):
        values = dict(attrs)
        self.tags.append((tag, values))
        if values.get("id"):
            self.ids.add(values["id"])


class ReportTemplateTest(unittest.TestCase):
    def parse(self):
        parser = StructureParser()
        parser.feed(REPORT.read_text())
        return parser

    def test_progressive_report_landmarks_exist(self):
        report = self.parse()
        self.assertTrue(
            {
                "run-status",
                "outcome-status",
                "attention-status",
                "your-action",
                "at-a-glance",
                "scope-updates",
                "outcomes",
                "unresolved",
                "deliverables",
                "decisions",
                "usage-models",
                "milestones",
            }.issubset(report.ids)
        )

    def test_visual_evidence_requires_interpretation_and_provenance(self):
        report = self.parse()
        evidence = [
            attrs
            for tag, attrs in report.tags
            if tag == "figure" and attrs.get("data-kind") == "Evidence"
        ]
        images = [attrs for tag, attrs in report.tags if tag == "img"]
        classes = {
            name
            for _, attrs in report.tags
            for name in attrs.get("class", "").split()
        }
        self.assertTrue(evidence)
        self.assertTrue(all(item.get("data-revision") for item in evidence))
        self.assertTrue(all(item.get("data-captured-at") for item in evidence))
        self.assertTrue(all(image.get("alt") for image in images))
        self.assertTrue({"observation", "acceptance", "limitation"}.issubset(classes))
        self.assertIn("figcaption", {tag for tag, _ in report.tags})

    def test_report_is_offline_and_javascript_free(self):
        report = self.parse()
        tags = {tag for tag, _ in report.tags}
        media = [
            value
            for tag, attrs in report.tags
            for key, value in attrs.items()
            if tag in {"img", "video", "source"} and key in {"src", "poster"}
        ]
        self.assertNotIn("script", tags)
        self.assertIn("details", tags)
        self.assertTrue(media)
        self.assertTrue(
            all(not value.startswith(("http://", "https://", "/", "data:")) for value in media)
        )


if __name__ == "__main__":
    unittest.main()
