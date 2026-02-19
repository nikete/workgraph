// Organizational Patterns & Formal Models for Workgraph
// Compiled from research by the workgraph identity

#set document(
  title: "Organizational Patterns & Formal Models for Workgraph",
  author: "The Workgraph Project",
)

#set text(font: "New Computer Modern", size: 11pt)
#set par(justify: true)
#set heading(numbering: "1.1")

// Title page
#page(numbering: none)[
  #v(4fr)
  #align(center)[
    #text(size: 28pt, weight: "bold")[Organizational Patterns]
    #v(4pt)
    #text(size: 28pt, weight: "bold")[& Formal Models]
    #v(12pt)
    #text(size: 16pt)[for Workgraph]
    #v(24pt)
    #text(size: 12pt, style: "italic")[
      A mathematics of organizations mapped onto task graph primitives
    ]
    #v(16pt)
    #text(size: 10pt)[February 2026]
  ]
  #v(6fr)
]

// Table of contents
#page(numbering: none)[
  #outline(title: "Contents", depth: 2, indent: auto)
]

// Start page numbering
#set page(numbering: "1")
#counter(page).update(1)

#include "organizational-patterns.typ"
