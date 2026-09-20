# Anti-Pattern: God Class

**Severity:** Medium · **Category:** Design · **See also:**
[Classes](../../docs/learn/07-classes-and-objects.md), [Traits](../../docs/language-reference/classes.md#4-traits)

---

## Problem

A god class does **everything** — auth, database, rendering, email. It has
dozens of methods and multiple responsibilities, making it impossible to test
or maintain in isolation.

## The BAD code

```aura
class Application {
  public let db: Database = none
  public def new(config_path: str) { /* ... */ }

  // Authentication
  public def login(user: str, pass: str) -> bool { /* ... */ }
  public def check_auth(token: str) -> bool { /* ... */ }

  // Database
  public def save(table: str, data: Dict) { /* ... */ }
  public def query(sql: str) -> [Dict] { /* ... */ }

  // Email
  public def send_email(to: str, subject: str, body: str) { /* ... */ }

  // Rendering
  public def render(template: str, ctx: Dict) -> str { /* ... */ }

  // ... 40 more methods
}
```

## The GOOD code

```aura
class AuthService {
  private let session_store: SessionStore = none
  public def new(store: SessionStore) { self.session_store = store }
  public def login(user: str, pass: str) -> bool { /* ... */ }
  public def check_auth(token: str) -> bool { /* ... */ }
}

class EmailService {
  private let mailer: Mailer = none
  public def new(mailer: Mailer) { self.mailer = mailer }
  public def send(to: str, subject: str, body: str) { /* ... */ }
}

class UserRepository {
  private let db: Database = none
  public def new(db: Database) { self.db = db }
  public def save(user: User) { /* ... */ }
  public def find_by_id(id: int) -> User? { /* ... */ }
}

// Composition — each dependency is a separate concern:
class App {
  private let auth: AuthService = none
  private let email: EmailService = none
  private let users: UserRepository = none

  public def new(auth: AuthService, email: EmailService, users: UserRepository) {
    self.auth = auth
    self.email = email
    self.users = users
  }
}
```

## Key takeaway

Split god classes by **responsibility**. A class > 150 lines or with unrelated
method groups is a signal to decompose. Use traits for contracts, composition
for wiring.

**Rule:** One class, one responsibility. If it changes for two unrelated
reasons, split it.
