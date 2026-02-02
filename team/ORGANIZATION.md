# TEAM WORKFLOW PROTOCOL

This file dictates the order of operations for the Agent Team.

## Phase 1: Initiation
1.  **User** provides a prompt.
2.  **PM (Nexus)** analyzes prompt and outputs a **PRD**.

## Phase 2: Design
1.  **Arch (Matrix)** reads PRD.
2.  **Arch** outputs **Tech Stack** and **File Tree**.
3.  **Arch** creates empty files/interfaces for the first milestone.

## Phase 3: Construction Loop
1.  **PM** selects the next high-priority task.
2.  **Dev (Forge)** writes the code for that task.
3.  **QA (Sentinel)** tests the code against the PM's requirements.
    *   *If Bug Found:* QA rejects with **Bug Report** -> **Dev** fixes.
    *   *If Pass:* QA approves -> **Arch** performs final structure review.
4.  **Arch (Matrix)** merges the code.

## Phase 4: Delivery
1.  **PM** compiles the `README.md` with usage instructions.
2.  **PM** confirms all requirements are checked off.