# CS2023 Topic Map

Open this package to browse all 17 knowledge areas and 166 entries under **Knowledge Units** in the final CS2023 report. Each unit opens its own citation to the publisher’s section. The five platform-specific SPD-SEP entries retain their distinct official identifiers. The report, published by the ACM, IEEE Computer Society and AAAI Joint Task Force, supplies the names, codes and ordering. [Official knowledge areas](https://csed.acm.org/knowledge-areas/) · [Report](https://csed.acm.org/wp-content/uploads/2025/11/CS2023-Report.htm)

## Authored coverage

| Authored prompt | Mapped knowledge units |
| --- | --- |
| Images and preimages | MSF-Discrete |
| Induction and invariants | MSF-Discrete |
| What does insertion move? | AL-Foundational |
| Sum types and case analysis | FPL-Types, FPL-Functional |

Qune courseware contributors wrote these four prompts. The other 162 indexed units are available for authors to add prompts. All 166 publisher links are citations. This reading Tablet assigns no grades. The connected lesson separately binds checks to an authored exercise. [Exact unit inventory and prompts](topics.toml) · [Connected lesson placements](../connected-lessons/learning-map.toml)

## Reproduce the inventory

Download the official report HTML and pass it to the Python 3.11+ verifier. It checks the publisher bytes against SHA-256 `8fc7212255af71eb4458122ccb8a93052601e6a2fff59d7c2f4e2db18081b72e`, extracts the linked table of contents and compares all area and unit codes, titles, order and citation anchors to the local inventory.

```sh
curl -fLsS https://csed.acm.org/wp-content/uploads/2025/11/CS2023-Report.htm -o /tmp/CS2023-Report.htm
python3 checks/inventory.py /tmp/CS2023-Report.htm
```

Update the package revision in `course.toml` to the SHA-256 of the exact `topics.toml` bytes after editing the map. Existing authored topic IDs are stable links.
