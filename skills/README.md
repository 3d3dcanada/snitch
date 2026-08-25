# Skills

Drop-in agent skills for `imprint`.

## Claude Code

    cp -r claude-code/imprint ~/.claude/skills/

Then it triggers on its own whenever a task involves image metadata, photo credit, GPS in photos,
C2PA, or preparing pictures for upload.

## Kilo Code

    cp kilo-code/imprint.md ~/.config/kilo/agents/

## Anything else

Both files are plain markdown with YAML frontmatter. The body is the whole instruction set, so it
ports to any agent runner that takes a system prompt.
