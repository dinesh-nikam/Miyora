# PRD: Codename “NekoForge” — Autonomous 3D Agent Pet for Codebases

## 1. Vision

NekoForge is a persistent, emotionally expressive 3D AI pet that lives inside a developer’s
codebase. Unlike a normal chatbot, it has a visible body, memory, personality, goals, and the
ability to use developer tools to help build software.

The pet is not only a companion. It is a proactive coding teammate with a memorable identity.

## 2. Problem

Current AI coding tools are powerful but impersonal:

- They disappear after each task.
- They do not remember project context well.
- They provide no emotional feedback or presence.
- They require the developer to constantly prompt them.
- They do not feel like a teammate.

Developers need an assistant that is useful, persistent, proactive, and enjoyable to work with.

## 3. Target User

- Solo developers
- Indie game developers
- Open-source maintainers
- Students learning to code
- Small startup engineering teams

## 4. Core Value Proposition

“Your codebase gets a living teammate: a 3D agent pet that remembers your project, understands
your work, uses tools safely, and helps you ship faster.”

## 5. Product Principles

1. Useful before cute: every animation and personality trait must support real work.
2. Permission-first: the pet never performs dangerous actions without approval.
3. Memory with control: users can inspect, edit, and delete everything the pet remembers.
4. Original identity: generated avatars and characters must avoid copyrighted characters.
5. Extensible: any developer can add tools, personalities, animations, and pet species.

## 6. Key Features

### 6.1 3D Agent Pet Identity

- Generate an original 3D anime, creature, robot, or mascot avatar from text or reference image.
- Use Tripo or Meshy API for text-to-3D and image-to-3D generation.
- Export and store the avatar as GLB; optionally support VRM for humanoid pets.
- Persist:
  - Pet name
  - Species/type
  - Personality
  - Voice
  - Appearance
  - Level
  - Unlockable accessories
- The same pet returns in every session.

### 6.2 Codebase Awareness

The pet can index and understand:

- Repository structure
- README and documentation
- Package dependencies
- Git branches and commit history
- Open TODO/FIXME comments
- Test results
- Build errors
- Linting issues
- Recent file changes

It maintains a project knowledge base called the “Codebase Mind.”

### 6.3 Autonomous Agent Mode

The pet can complete tasks through a plan-act-observe loop:

1. Understand the request.
2. Create a task plan.
3. Ask for approval if needed.
4. Use tools.
5. Observe results.
6. Fix errors.
7. Report what it did.

Example tasks:

- “Find why login tests fail.”
- “Add API documentation for the payments module.”
- “Refactor this function and run tests.”
- “Create a changelog from the last 10 commits.”
- “Review this pull request.”
- “Generate a component from this Figma screenshot.”

### 6.4 Tool System

The pet uses MCP-style tools:

- File read/write/search
- Terminal command execution
- Git operations
- GitHub issues and pull requests
- Test runner
- Linter/formatter
- Browser research
- Documentation search
- Database inspection
- Deployment status
- Design-file import

Every tool has a permission level:

| Level | Behavior |
|---|---|
| Read | Allowed automatically |
| Low risk | Allowed with notification |
| Medium risk | Requires user confirmation |
| High risk | Requires explicit approval |

High-risk actions include deleting files, pushing to main, installing packages, running shell
commands, accessing secrets, and deploying.

### 6.5 Memory System

Use four memory layers:

1. **Working memory:** Current task, files opened, recent commands, active errors.
2. **Episodic memory:** What happened in previous sessions.
3. **Semantic memory:** Stable project facts, architecture decisions, user preferences.
4. **Procedural memory:** Learned workflows, such as how this project runs tests or deploys.

Users can view, edit, pin, or delete memories.

### 6.6 Emotion and Personality Engine

The pet has an internal state:

- Mood
- Energy
- Confidence
- Focus
- Trust level
- Current goal

Events affect emotions:

| Event | Reaction |
|---|---|
| Tests pass | Celebrates |
| Tests fail | Worried, suggests diagnosis |
| User praises pet | Happy, increased trust |
| Build breaks | Alert and focused |
| Long idle period | Sleeps or explores codebase |
| New dependency added | Curious, asks why |
| Dangerous command requested | Cautious, asks for confirmation |

### 6.7 Proactive Assistance

The pet can notice opportunities and suggest actions:

- “Three tests started failing after your last commit. Want me to investigate?”
- “This function has no tests. Should I write one?”
- “I found five TODOs related to authentication. Want a cleanup plan?”
- “Your README does not explain how to run the project. Want me to update it?”
- “This dependency has a known vulnerability. Want me to research a fix?”

The user controls proactivity: Off, Gentle, Balanced, or Autonomous.

### 6.8 Voice and Animation

- Speech-to-text input
- Text-to-speech output
- Lip sync
- Idle animations
- Emotional expressions
- Celebration animations
- Thinking animation
- Sleeping animation
- Error reaction animation
- Task-progress animation

### 6.9 Pet Growth System

The pet earns XP for useful completed work:

- Fixing a bug
- Writing tests
- Improving documentation
- Completing a reviewed task
- Helping debug an issue
- Maintaining a clean git history

XP unlocks:

- New outfits
- Accessories
- Animations
- Voice styles
- New tool permissions
- Special abilities, such as “Deep Code Review” or “Night Debugger”

This makes the pet feel alive without turning the product into a game first.

### 6.10 Privacy and Safety

- Local-first memory option
- Explicit opt-in for screen awareness
- No automatic access to secrets
- Secret redaction in logs
- Full action audit trail
- Undo support where possible
- Clear distinction between suggestion and completed action
- User can pause, mute, or delete the pet at any time

## 7. User Stories

1. As a developer, I want my pet to remember my project so I do not need to explain it again.
2. As a developer, I want the pet to detect failing tests and offer to fix them.
3. As a developer, I want to approve risky actions before the pet performs them.
4. As a developer, I want the pet to show emotion so I can quickly understand task status.
5. As a developer, I want to create an original 3D pet from a text description.
6. As a developer, I want to give my pet tools through MCP.
7. As a developer, I want to delete any memory the pet has stored.
8. As a team lead, I want pets to specialize in review, testing, or documentation.

## 8. MVP Scope

### Must Have

- Text chat with pet
- Original 3D avatar generation
- Avatar loading and preview
- Basic emotion states
- Codebase file search
- Read-only repository analysis
- Task planning
- Tool permissions
- Test-runner integration
- Memory storage
- Basic voice output
- Audit log
- GLB export/download

### Should Have

- Git and GitHub integration
- Proactive suggestions
- Edit/write permissions with approval
- Lip sync
- VRM support
- Pet customization
- XP and unlock system

### Later

- Multi-pet collaboration
- Team workspaces
- Mobile companion app
- Plugin marketplace
- Local-only LLM mode
- AR/VR presence

## 9. Technical Architecture

### Frontend

- Desktop app: Electron or Tauri
- 3D rendering: Three.js, React Three Fiber, Babylon.js, or Unity
- Avatar formats: GLB primary, VRM optional
- UI: chat panel, task panel, memory panel, permissions panel, pet viewport

### Backend / Agent Layer

- Agent orchestrator
- Planner
- Tool router
- Memory manager
- Emotion engine
- Permission manager
- Task queue
- Audit logger

### 3D Generation

- Tripo API for text-to-3D and image-to-3D
- Optional Meshy API
- Blender Python for cleanup, rigging checks, rendering, and customization
- Store generated assets in object storage or local `/output/models`

### Memory

- Vector database for semantic search
- SQLite/Postgres for structured memory
- File-based project index
- Embedding model for code and conversation retrieval

### Integrations

- GitHub
- Git
- Terminal
- VS Code / JetBrains
- Browser
- MCP servers
- CI/CD systems

## 10. Success Metrics

- Weekly active pets
- Tasks completed without user correction
- Average time saved per task
- Percentage of proactive suggestions accepted
- Memory retention quality
- Test-fix success rate
- User retention after 7 and 30 days
- Number of tools installed per user

## 11. Risks

| Risk | Mitigation |
|---|---|
| Pet performs unsafe action | Permission levels, confirmation, audit trail |
| Generated avatar resembles copyrighted character | Block named characters, use original-design prompts, moderation |
| Memory becomes inaccurate | Editable memory, source links, expiration rules |
| Agent makes bad code changes | Sandbox, diffs, tests, rollback, human approval |
| Too gimmicky | Focus on measurable developer productivity |
| API costs | Free-tier testing, caching, user-supplied keys |

## 12. Launch Strategy

### Phase 1: Pet Shell
Create the 3D pet, chat, memory, emotions, and local project awareness.

### Phase 2: Coding Teammate
Add repository indexing, test integration, task planning, and safe tool use.

### Phase 3: Autonomous Helper
Add proactive monitoring, GitHub workflows, CI awareness, and multi-step task execution.

### Phase 4: Pet Society
Enable pet customization, growth, plugin tools, and multi-pet collaboration.

## 13. One-Line Pitch

**NekoForge turns your codebase into a home for an original 3D AI pet that remembers your project, feels your progress, uses developer tools safely, and helps you ship.**