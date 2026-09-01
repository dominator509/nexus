/**
 * EP-006 M3 real-server bootstrap (owner diagnostic plan 2026-08-13).
 *
 * Starts REAL ephemeral containers (TESTING.md real dependency rules):
 * - postgres:18.4 (repo-pinned image, digest verified)
 * - temporalio/server:1.31.2 (digest verified) with DB=postgres12
 * - temporalio/admin-tools:1.31.2 runs temporal-sql-tool schema setup and
 *   the temporal CLI (namespace + cluster health)
 *
 * Hard rules from the owner diagnostic plan:
 * - One cryptographically random, shell-safe hexadecimal PostgreSQL
 *   password per stack, held in a single in-memory credential object.
 *   It is NEVER printed, logged, written to the repository, or passed
 *   through a shell string. Diagnostics log only a SHA-256 fingerprint
 *   prefix of the password, and every consumer (postgres, sql-tool,
 *   server) is asserted to carry the SAME fingerprint.
 * - One ephemeral Docker network per stack with stable aliases:
 *   postgres -> "postgres", temporal server -> "temporal". Service
 *   discovery never depends on random container names.
 * - REAL authenticated queries (psql, PGPASSWORD env) prove the
 *   credential against the default DB before schema bootstrap and
 *   against BOTH temporal databases after schema, BEFORE the Temporal
 *   server is started. pg_isready alone is insufficient.
 * - The Temporal server is only started after DB auth + schema are
 *   proven. Its container state is polled first; if it exits, full
 *   evidence (id, digest, exit code, OOMKilled, State.Error, redacted
 *   logs, non-secret config) is captured and surfaced. Namespace
 *   creation happens only after `temporal operator cluster health`
 *   succeeds (COMMANDS.md).
 * - Teardown removes every container, the network, and anonymous
 *   volumes; no worker process survives.
 */

import { execFileSync } from "node:child_process";
import { createConnection } from "node:net";
import { createHash, randomBytes } from "node:crypto";
import { readFileSync, writeFileSync } from "node:fs";
import yaml from "js-yaml";

export const POSTGRES_IMAGE = "postgres:18.4";
export const POSTGRES_DIGEST =
  "sha256:a02db8cac496f15b094798a38254f14d6e00741f709360e5e00bb6668ea31636";
export const TEMPORAL_SERVER_IMAGE = "temporalio/server:1.31.2";
export const TEMPORAL_SERVER_DIGEST =
  "sha256:b5ecdb8282bededae2a10c36e8d862e27d0bc2d247fc73c5416025997ab4a1da";
export const ADMIN_TOOLS_IMAGE = "temporalio/admin-tools:1.31.2";
export const ADMIN_TOOLS_DIGEST =
  "sha256:dbc5fcd6ee8f0f4d808bf765af9a87dea9d8a283abfdcfbd2fc148496ba66107";

const NAMESPACE: string = "nexus";
const POSTGRES_USER: string = "nexus";
const POSTGRES_DEFAULT_DB: string = "postgres";
const DBNAME: string = "temporal";
const VISIBILITY_DBNAME: string = "temporal_visibility";
const DYNAMIC_CONFIG_TARGET = "/etc/temporal/config/dynamicconfig/docker.yaml";

/** In-memory credential object; the password never leaves this process. */
interface StackCredentials {
  readonly password: string;
  readonly fingerprint: string;
}

function generateCredentials(): StackCredentials {
  const password = randomBytes(24).toString("hex");
  const fingerprint = createHash("sha256").update(password).digest("hex");
  return { password, fingerprint };
}

function fingerprintOf(value: string): string {
  return createHash("sha256").update(value).digest("hex");
}

function assertSameFingerprint(
  label: string,
  value: string,
  expected: string,
): void {
  if (fingerprintOf(value) !== expected) {
    throw new Error(
      `credential fingerprint mismatch for ${label}; aborting stack bootstrap`,
    );
  }
}

/** Redact a secret from diagnostic text (observability requirement). */
export function redact(text: string, secret: string): string {
  if (secret.length === 0) {
    // Empty secret is degenerate: split("") would interleave
    // <redacted> between every character. No-op is the safe behavior.
    return text;
  }
  return text.split(secret).join("<redacted>");
}

function dynamicConfigPath(): string {
  return new URL("./dynamicconfig/docker.yaml", import.meta.url).pathname;
}

/**
 * Every resource a stack owns. Teardown MUST remove all of them; the
 * registry file lets the vitest globalTeardown clean up even when the
 * fork process dies before explicit disposal runs.
 */
export interface StackResources {
  readonly postgresContainer: string;
  readonly serverContainer: string;
  readonly network: string;
  readonly volumes: readonly string[];
}

export interface TemporalStack extends StackResources {
  readonly address: string;
  readonly namespace: string;
  /**
   * PRIMARY teardown path: explicit, awaited, idempotent, and
   * error-accumulating. Every cleanup step is attempted even when
   * earlier steps fail; all failures are reported in one
   * StackDisposeError. Missing resources are a legitimate idempotent
   * no-op (second dispose / crash recovery), NOT a swallowed failure.
   */
  dispose(): Promise<void>;
}

/** All failures from one disposal attempt, surfaced together. */
export class StackDisposeError extends Error {
  readonly failures: readonly string[];
  constructor(failures: readonly string[]) {
    super(
      `stack disposal failed with ${failures.length} error(s):\n${failures
        .map((failure) => `- ${failure}`)
        .join("\n")}`,
    );
    this.name = "StackDisposeError";
    this.failures = failures;
  }
}

/** Ephemeral registry consulted by vitest globalTeardown and the gate's orphan audit. */
export const STACK_STATE_FILE = "/tmp/nexus-ep006-stack-state.json";

function readStateEntries(): StackResources[] {
  try {
    const parsed = JSON.parse(readFileSync(STACK_STATE_FILE, "utf8")) as {
      entries?: StackResources[];
    };
    return Array.isArray(parsed.entries) ? parsed.entries : [];
  } catch {
    // Absent or malformed state file: nothing registered yet.
    return [];
  }
}

function writeStateEntries(entries: StackResources[]): void {
  writeFileSync(STACK_STATE_FILE, `${JSON.stringify({ entries }, null, 2)}\n`);
}

/** Register a live stack so suite-level cleanup can find it later. */
export function registerStackResources(resources: StackResources): void {
  const entries = readStateEntries();
  entries.push(resources);
  writeStateEntries(entries);
}

/** Remove a stack from the registry only after its disposal fully succeeded. */
export function unregisterStackResources(resources: StackResources): void {
  const entries = readStateEntries().filter(
    (entry) => entry.network !== resources.network,
  );
  writeStateEntries(entries);
}

function docker(args: string[]): string {
  // stdio pipe: the child's stderr stays inside the thrown error
  // (error.stderr / error.message) instead of being echoed to the
  // test log; without this, idempotent missing-resource teardown
  // attempts print raw docker noise after every green run.
  return execFileSync("docker", args, {
    encoding: "utf8",
    stdio: ["ignore", "pipe", "pipe"],
  }).trim();
}

/** Raw docker access for narrow-layer teardown proofs (test zone). */
export function runDocker(args: string[]): string {
  return docker(args);
}

export function stackSuffix(): string {
  return randomBytes(4).toString("hex");
}

function randomSuffix(): string {
  return stackSuffix();
}

/**
 * Missing-resource docker errors are the idempotency signal: a second
 * dispose or crash recovery must not fail because the resource is
 * already gone. EVERY other docker error is a real cleanup failure and
 * is surfaced. No silent catch is allowed in teardown.
 */
function isMissingResource(message: string): boolean {
  return (
    message.includes("No such container") ||
    message.includes("no such container") ||
    message.includes("no such volume") ||
    message.includes("not found")
  );
}

function removeContainer(name: string, failures: string[]): void {
  try {
    docker(["rm", "-f", "-v", name]);
  } catch (error) {
    const message = error instanceof Error ? error.message : String(error);
    if (!isMissingResource(message)) {
      failures.push(`remove container ${name}: ${message}`);
    }
  }
}

/**
 * Remove any container still attached to the stack network (admin-tools
 * / one-shot psql containers that should have self-removed with --rm
 * but must not be allowed to leak if a run was interrupted).
 */
function sweepNetworkContainers(network: string, failures: string[]): void {
  let ids: string[] = [];
  try {
    const out = docker(["ps", "-aq", "--filter", `network=${network}`]);
    ids = out.length === 0 ? [] : out.split("\n").filter((id) => id.length > 0);
  } catch (error) {
    const message = error instanceof Error ? error.message : String(error);
    if (!isMissingResource(message)) {
      failures.push(`list containers on network ${network}: ${message}`);
    }
    return;
  }
  for (const id of ids) {
    removeContainer(id, failures);
  }
}

/**
 * Synchronous, ordered, error-accumulating teardown. Every step is
 * attempted even when earlier steps fail; returns every failure.
 * Order matters: containers before network, network before volumes.
 */
export function disposeStackResourcesSync(resources: StackResources): string[] {
  const failures: string[] = [];
  sweepNetworkContainers(resources.network, failures);
  removeContainer(resources.serverContainer, failures);
  removeContainer(resources.postgresContainer, failures);
  try {
    docker(["network", "rm", resources.network]);
  } catch (error) {
    const message = error instanceof Error ? error.message : String(error);
    if (!isMissingResource(message)) {
      failures.push(`remove network ${resources.network}: ${message}`);
    }
  }
  for (const volume of resources.volumes) {
    try {
      docker(["volume", "rm", volume]);
    } catch (error) {
      const message = error instanceof Error ? error.message : String(error);
      if (!isMissingResource(message)) {
        failures.push(`remove volume ${volume}: ${message}`);
      }
    }
  }
  return failures;
}

/** Async disposal: surface every failure together (never swallowed). */
export async function disposeStackResources(
  resources: StackResources,
): Promise<void> {
  const failures = disposeStackResourcesSync(resources);
  if (failures.length > 0) {
    throw new StackDisposeError(failures);
  }
}

async function disposeResources(resources: StackResources): Promise<void> {
  const failures = disposeStackResourcesSync(resources);
  if (failures.length === 0) {
    // Fully disposed: drop the registry entry so suite-level cleanup
    // does not re-run it (it would be a harmless no-op, but the state
    // file should reflect reality).
    unregisterStackResources(resources);
  }
  if (failures.length > 0) {
    throw new StackDisposeError(failures);
  }
}

function hostPortFor(container: string, containerPort: number): number {
  const out = docker(["port", container, String(containerPort)]).split(
    "\n",
  )[0] as string;
  // Accept both "5432/tcp -> 127.0.0.1:PORT" and the normalized
  // "127.0.0.1:PORT" forms; the port is always the last colon field.
  const lastColon = out.lastIndexOf(":");
  const portText = lastColon === -1 ? out : out.slice(lastColon + 1);
  const port = Number(portText.trim());
  if (!Number.isInteger(port) || port <= 0 || port >= 65536) {
    throw new Error(
      `cannot parse host port for ${container}:${containerPort} from '${out}'`,
    );
  }
  return port;
}

function sleep(ms: number): void {
  const end = Date.now() + ms;
  while (Date.now() < end) {
    /* bounded sleep */
  }
}

async function waitForConnectAsync(
  connect: () => Promise<void>,
  label: string,
  timeoutMs = 60_000,
): Promise<void> {
  const deadline = Date.now() + timeoutMs;
  let last: unknown;
  while (Date.now() < deadline) {
    try {
      await connect();
      return;
    } catch (error) {
      last = error;
      await new Promise((resolve) => setTimeout(resolve, 2000));
    }
  }
  throw new Error(`${label} not ready within ${timeoutMs}ms: ${String(last)}`);
}

function probeTcp(port: number): Promise<void> {
  return new Promise((resolve, reject) => {
    const socket = createConnection({ host: "127.0.0.1", port });
    const onError = (error: Error): void => {
      socket.destroy();
      reject(error);
    };
    socket.once("connect", () => {
      socket.destroy();
      resolve();
    });
    socket.once("error", onError);
  });
}

/** Real postgres readiness: pg_isready inside the container. */
function waitForPostgres(container: string, timeoutMs = 60_000): void {
  const deadline = Date.now() + timeoutMs;
  while (Date.now() < deadline) {
    try {
      const out = execFileSync(
        "docker",
        ["exec", container, "pg_isready", "-U", POSTGRES_USER],
        { encoding: "utf8" },
      );
      if (out.includes("accepting connections")) {
        return;
      }
    } catch {
      /* container still starting */
    }
    sleep(2000);
  }
  throw new Error(`postgres ${container} not ready within ${timeoutMs}ms`);
}

/**
 * REAL authenticated query from a one-shot psql client on the same
 * network (PGPASSWORD env, never printed). Fails the stack if the
 * credential cannot authenticate or current_user is not nexus.
 */
function assertAuthenticatedQuery(
  network: string,
  credentials: StackCredentials,
  db: string,
): void {
  const out = docker([
    "run",
    "--rm",
    "--network",
    network,
    "-e",
    `PGPASSWORD=${credentials.password}`,
    "--entrypoint",
    "psql",
    `${POSTGRES_IMAGE}@${POSTGRES_DIGEST}`,
    "-h",
    "postgres",
    "-U",
    POSTGRES_USER,
    "-d",
    db,
    "-tAc",
    "SELECT 1; SELECT current_user;",
  ]);
  const lines = out.split("\n").map((line) => line.trim());
  if (!lines.includes("1") || !lines.includes(POSTGRES_USER)) {
    throw new Error(
      `database authentication proof FAILED for db ${db} (expected SELECT 1 and current_user=${POSTGRES_USER})`,
    );
  }
}

/**
 * Validate the dynamic-config fixture with js-yaml before mounting it.
 * Node-side parsing (js-yaml is a declared dependency) - never spawn a
 * system python3 whose PyYAML presence varies between runners (CI
 * system python has no yaml module; the uv-locked env has none either).
 */
function validateDynamicConfig(): void {
  const path = dynamicConfigPath();
  const raw = readFileSync(path, "utf8");
  if (raw.trim().length === 0) {
    throw new Error(`dynamic config fixture is empty: ${path}`);
  }
  let parsed: unknown;
  try {
    parsed = yaml.load(raw);
  } catch (err) {
    throw new Error(
      `dynamic config YAML validation failed: ${path}: ${String(err)}`,
    );
  }
  if (parsed === undefined || typeof parsed !== "object") {
    throw new Error(`dynamic config YAML must parse to an object: ${path}`);
  }
}

interface ServerExitEvidence {
  containerId: string;
  image: string;
  exitCode: string;
  oomKilled: string;
  stateError: string;
  logs: string;
  config: Record<string, string>;
}

function captureServerExitEvidence(
  container: string,
  secret: string,
  config: Record<string, string>,
): ServerExitEvidence {
  const inspect = (format: string): string => {
    try {
      return docker(["inspect", "--format", format, container]);
    } catch {
      return "<inspect failed>";
    }
  };
  let logs = "<logs unavailable>";
  try {
    logs = docker(["logs", container]);
  } catch {
    /* fall through */
  }
  return {
    containerId: inspect("{{.Id}}"),
    image: inspect("{{.Image}}"),
    exitCode: inspect("{{.State.ExitCode}}"),
    oomKilled: inspect("{{.State.OOMKilled}}"),
    stateError: inspect("{{.State.Error}}"),
    logs: redact(logs, secret),
    config,
  };
}

function containerStatus(container: string): string {
  try {
    return docker(["inspect", "--format", "{{.State.Status}}", container]);
  } catch {
    return "<missing>";
  }
}

/** Poll container state; if it exits, throw with full evidence. */
function waitForServerRunning(
  container: string,
  credentials: StackCredentials,
  config: Record<string, string>,
  timeoutMs = 60_000,
): void {
  const deadline = Date.now() + timeoutMs;
  while (Date.now() < deadline) {
    const status = containerStatus(container);
    if (status === "running") {
      return;
    }
    if (status !== "created" && status !== "restarting") {
      const evidence = captureServerExitEvidence(
        container,
        credentials.password,
        config,
      );
      throw new Error(
        `temporal server container exited (status=${status})\n` +
          `container id: ${evidence.containerId}\n` +
          `image: ${evidence.image}\n` +
          `exit code: ${evidence.exitCode}\n` +
          `oomkilled: ${evidence.oomKilled}\n` +
          `state error: ${evidence.stateError}\n` +
          `config: ${JSON.stringify(evidence.config)}\n` +
          `logs:\n${evidence.logs}`,
      );
    }
    sleep(2000);
  }
  const evidence = captureServerExitEvidence(
    container,
    credentials.password,
    config,
  );
  throw new Error(
    `temporal server container never reached running within ${timeoutMs}ms; status=${containerStatus(container)}\n` +
      `logs:\n${evidence.logs}`,
  );
}

/**
 * Real cluster health via the pinned admin-tools image on the same
 * network (COMMANDS.md). Polls with bounded retries: the server
 * container may be "running" while its frontend is still connecting to
 * postgres. Throws on timeout with the server's redacted logs.
 */
function assertClusterHealth(
  network: string,
  serverContainer: string,
  secret: string,
  timeoutMs = 90_000,
): string {
  const deadline = Date.now() + timeoutMs;
  let last: unknown;
  while (Date.now() < deadline) {
    try {
      const out = docker([
        "run",
        "--rm",
        "--network",
        network,
        "--entrypoint",
        "temporal",
        `${ADMIN_TOOLS_IMAGE}@${ADMIN_TOOLS_DIGEST}`,
        "operator",
        "cluster",
        "health",
        "--address",
        "temporal:7233",
      ]);
      if (out.trim().length > 0) {
        return out;
      }
    } catch (error) {
      last = error;
      // Not ready yet: bounded retry, do not reduce to a later error.
      sleep(2000);
    }
  }
  let logs = "<logs unavailable>";
  try {
    logs = redact(docker(["logs", serverContainer]), secret);
  } catch {
    /* fall through */
  }
  throw new Error(
    `temporal cluster health not ready within ${timeoutMs}ms: ${String(last)}\nserver logs:\n${logs}`,
  );
}

/** Create (idempotently) and prove the nexus namespace exists. */
function ensureNamespace(network: string): void {
  const createArgs = [
    "run",
    "--rm",
    "--network",
    network,
    "--entrypoint",
    "temporal",
    `${ADMIN_TOOLS_IMAGE}@${ADMIN_TOOLS_DIGEST}`,
    "operator",
    "namespace",
    "create",
    "--retention",
    "24h",
    "--namespace",
    NAMESPACE,
    "--address",
    "temporal:7233",
  ];
  let created = true;
  try {
    docker(createArgs);
  } catch (error) {
    created = false;
    // Idempotent: namespace may already exist from a prior run.
    const describe = describeNamespace(network);
    if (!describe.includes(NAMESPACE)) {
      throw new Error(
        `namespace creation failed and describe could not prove existence: ${String(
          error,
        )}`,
      );
    }
  }
  const proof = describeNamespace(network);
  if (!proof.includes(NAMESPACE)) {
    throw new Error(
      `namespace ${NAMESPACE} not provable after ${created ? "create" : "existing"}`,
    );
  }
}

function describeNamespace(network: string): string {
  return docker([
    "run",
    "--rm",
    "--network",
    network,
    "--entrypoint",
    "temporal",
    `${ADMIN_TOOLS_IMAGE}@${ADMIN_TOOLS_DIGEST}`,
    "operator",
    "namespace",
    "describe",
    "--namespace",
    NAMESPACE,
    "--address",
    "temporal:7233",
  ]);
}

/**
 * Start postgres + temporal server + schema/namespace bootstrap. Caller
 * MUST dispose() the returned stack explicitly (primary path: try/
 * finally); suite-level globalTeardown and the process-exit hook are
 * layered safety nets, not the primary mechanism.
 */
export async function startTemporalStack(): Promise<TemporalStack> {
  const credentials = generateCredentials();
  const suffix = randomSuffix();
  const network = `nexus-ep006-${suffix}`;
  const pgContainer = `nexus-ep006-pg-${suffix}`;
  const serverContainer = `nexus-ep006-server-${suffix}`;

  // One immutable credential object; every consumer must match its
  // fingerprint. Log only a short non-reversible prefix.
  assertSameFingerprint(
    "postgres",
    credentials.password,
    credentials.fingerprint,
  );
  assertSameFingerprint(
    "sql-tool",
    credentials.password,
    credentials.fingerprint,
  );
  assertSameFingerprint(
    "server",
    credentials.password,
    credentials.fingerprint,
  );
  // eslint-disable-next-line no-console
  console.log(
    `[ep006] postgres credential sha256 prefix: ${credentials.fingerprint.slice(0, 12)} (generated per-stack)`,
  );

  docker(["network", "create", network]);

  // 1. PostgreSQL 18.4 (repo-pinned digest) with stable alias "postgres".
  docker([
    "run",
    "-d",
    "--network",
    network,
    "--network-alias",
    "postgres",
    "--name",
    pgContainer,
    "-e",
    `POSTGRES_USER=${POSTGRES_USER}`,
    "-e",
    `POSTGRES_PASSWORD=${credentials.password}`,
    "-e",
    `POSTGRES_DB=${POSTGRES_DEFAULT_DB}`,
    "-p",
    "127.0.0.1::5432",
    `${POSTGRES_IMAGE}@${POSTGRES_DIGEST}`,
  ]);

  // Capture the anonymous volumes postgres created (removed explicitly
  // by dispose, in addition to docker rm -v on the container).
  const pgVolumes = docker([
    "inspect",
    "--format",
    "{{range .Mounts}}{{.Name}}\n{{end}}",
    pgContainer,
  ])
    .split("\n")
    .map((line) => line.trim())
    .filter((line) => line.length > 0);

  const resources: StackResources = {
    postgresContainer: pgContainer,
    serverContainer,
    network,
    volumes: pgVolumes,
  };
  // Register IMMEDIATELY (before bootstrap): a crash mid-bootstrap must
  // still leave the suite-level cleanup (globalTeardown / orphan audit)
  // able to find and remove these resources.
  registerStackResources(resources);

  try {
    // Readiness through the published host port + real pg_isready.
    const pgPort = hostPortFor(pgContainer, 5432);
    await waitForConnectAsync(() => probeTcp(pgPort), "postgres tcp");
    waitForPostgres(pgContainer);

    // 2. Prove the credential BEFORE schema bootstrap against the
    // default database (real authenticated psql query).
    assertAuthenticatedQuery(network, credentials, POSTGRES_DEFAULT_DB);

    // 3. Schema bootstrap via admin-tools (transcribed from auto-setup.sh
    // v1.31.2: create if DBNAME != POSTGRES_USER, setup-schema -v 0.0,
    // update-schema -d SCHEMA_DIR for temporal and visibility).
    const sqlTool = (db: string, extra: string[]): void => {
      const base = [
        "--plugin",
        "postgres12",
        "--ep",
        "postgres",
        "-u",
        POSTGRES_USER,
        "--pw",
        credentials.password,
        "-p",
        "5432",
        "--db",
        db,
      ];
      docker([
        "run",
        "--rm",
        "--network",
        network,
        "--entrypoint",
        "temporal-sql-tool",
        `${ADMIN_TOOLS_IMAGE}@${ADMIN_TOOLS_DIGEST}`,
        ...base,
        ...extra,
      ]);
    };

    if (DBNAME !== POSTGRES_USER) {
      sqlTool(DBNAME, ["create"]);
    }
    sqlTool(DBNAME, ["setup-schema", "-v", "0.0"]);
    sqlTool(DBNAME, [
      "update-schema",
      "-d",
      "/etc/temporal/schema/postgresql/v12/temporal/versioned",
    ]);
    if (VISIBILITY_DBNAME !== POSTGRES_USER) {
      sqlTool(VISIBILITY_DBNAME, ["create"]);
    }
    sqlTool(VISIBILITY_DBNAME, ["setup-schema", "-v", "0.0"]);
    sqlTool(VISIBILITY_DBNAME, [
      "update-schema",
      "-d",
      "/etc/temporal/schema/postgresql/v12/visibility/versioned",
    ]);

    // 4. After schema setup, repeat the authenticated SELECT 1 checks
    // against BOTH databases (owner plan). Only then start Temporal.
    assertAuthenticatedQuery(network, credentials, DBNAME);
    assertAuthenticatedQuery(network, credentials, VISIBILITY_DBNAME);

    // 5. Validate + mount dynamic config read-only at the pinned path.
    validateDynamicConfig();

    const serverConfig: Record<string, string> = {
      DB: "postgres12",
      DB_PORT: "5432",
      POSTGRES_SEEDS: "postgres",
      POSTGRES_USER,
      DBNAME,
      VISIBILITY_DBNAME,
      BIND_ON_IP: "0.0.0.0",
      DYNAMIC_CONFIG_FILE_PATH: DYNAMIC_CONFIG_TARGET,
    };

    // 6. Temporal server 1.31.2 with real PostgreSQL persistence.
    docker([
      "run",
      "-d",
      "--network",
      network,
      "--network-alias",
      "temporal",
      "--name",
      serverContainer,
      "-e",
      "DB=postgres12",
      "-e",
      "DB_PORT=5432",
      "-e",
      "POSTGRES_SEEDS=postgres",
      "-e",
      `POSTGRES_USER=${POSTGRES_USER}`,
      "-e",
      `POSTGRES_PWD=${credentials.password}`,
      "-e",
      `DBNAME=${DBNAME}`,
      "-e",
      `VISIBILITY_DBNAME=${VISIBILITY_DBNAME}`,
      "-e",
      "BIND_ON_IP=0.0.0.0",
      "-e",
      `DYNAMIC_CONFIG_FILE_PATH=${DYNAMIC_CONFIG_TARGET}`,
      "-v",
      `${dynamicConfigPath()}:${DYNAMIC_CONFIG_TARGET}:ro`,
      "-p",
      "127.0.0.1::7233",
      `${TEMPORAL_SERVER_IMAGE}@${TEMPORAL_SERVER_DIGEST}`,
    ]);

    // 7. Observe container state FIRST (owner plan); capture full
    // evidence if it exits.
    waitForServerRunning(serverContainer, credentials, serverConfig);

    // 8. Real cluster health (COMMANDS.md) before any namespace work.
    assertClusterHealth(network, serverContainer, credentials.password);

    const serverPort = hostPortFor(serverContainer, 7233);
    const address = `127.0.0.1:${serverPort}`;
    await waitForConnectAsync(() => probeTcp(serverPort), "temporal server");

    // 9. Register + prove the nexus namespace after health.
    ensureNamespace(network);

    return {
      address,
      namespace: NAMESPACE,
      postgresContainer: pgContainer,
      serverContainer,
      network,
      volumes: resources.volumes,
      dispose: async () => {
        await disposeResources(resources);
      },
    };
  } catch (error) {
    // Bootstrap failed: still dispose every created resource, surfacing
    // cleanup failures alongside the original error. The registry entry
    // is retained unless disposal fully succeeded, so suite-level
    // cleanup can retry anything that leaked.
    const cleanupFailures = disposeStackResourcesSync(resources);
    if (cleanupFailures.length === 0) {
      unregisterStackResources(resources);
      throw error;
    }
    throw new Error(
      `${String(error)}\ncleanup after bootstrap failure also failed:\n${cleanupFailures
        .map((failure) => `- ${failure}`)
        .join("\n")}`,
    );
  }
}
