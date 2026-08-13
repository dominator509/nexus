/**
 * EP-006 M3 test session: one real server stack, one SDK Runtime, one
 * NativeConnection per PROCESS (EP-005 owner doctrine: the test process
 * owns the runtime and connection; the worker factory never creates its
 * own).
 *
 * Single-fork lifecycle (vitest.config.ts: singleFork + fileParallelism
 * false): all test files share ONE process, so the stack is started once
 * on first use. The PRIMARY teardown path is the explicit async
 * `await session.shutdown()` called from the suite's final teardown test
 * (try/finally pattern) - NOT a process-exit hook. Two layered safety
 * nets remain: the vitest globalTeardown (reads the stack registry file
 * and disposes every registered stack even if the fork dies) and this
 * module's process.once("exit") hook (last-resort sync docker cleanup
 * for a process that exits without either of the former running).
 *
 * shutdown() is idempotent: a second call is a no-op. Every failure is
 * accumulated and thrown - teardown failures are test failures.
 */

import { Runtime, NativeConnection } from "@temporalio/worker";
import { Client, Connection } from "@temporalio/client";

import {
  disposeStackResourcesSync,
  startTemporalStack,
  type StackResources,
  type TemporalStack,
} from "./stack.js";

export interface TestSession {
  readonly stack: TemporalStack;
  readonly address: string;
  readonly namespace: string;
  readonly runtime: Runtime;
  readonly connection: NativeConnection;
  readonly client: Client;
  /** PRIMARY teardown: explicit, awaited, idempotent, error-accumulating. */
  shutdown(): Promise<void>;
}

let cached: TestSession | undefined;
let disposed = false;

export async function getSession(): Promise<TestSession> {
  if (disposed) {
    throw new Error(
      "ep006 test session already disposed; no further tests may use it",
    );
  }
  if (cached !== undefined) {
    return cached;
  }
  const stack = await startTemporalStack();
  const runtime = Runtime.install({});
  const connection = await NativeConnection.connect({
    address: stack.address,
  });
  const sdkConnection = await Connection.connect({
    address: stack.address,
  });
  const client = new Client({
    connection: sdkConnection,
    namespace: stack.namespace,
  });
  const session: TestSession = {
    stack,
    address: stack.address,
    namespace: stack.namespace,
    runtime,
    connection,
    client,
    shutdown: async () => {
      if (disposed) {
        return;
      }
      disposed = true;
      const failures: string[] = [];
      try {
        await connection.close();
      } catch (error) {
        failures.push(`close NativeConnection: ${String(error)}`);
      }
      try {
        await sdkConnection.close();
      } catch (error) {
        failures.push(`close SDK Connection: ${String(error)}`);
      }
      try {
        await stack.dispose();
      } catch (error) {
        failures.push(`dispose stack: ${String(error)}`);
      }
      if (failures.length > 0) {
        throw new Error(
          `ep006 session shutdown failed:\n${failures
            .map((failure) => `- ${failure}`)
            .join("\n")}`,
        );
      }
    },
  };
  cached = session;

  // EMERGENCY net ONLY. The primary cleanup path is the explicit
  // `await session.shutdown()` (suite teardown test) plus vitest
  // globalTeardown; this hook only guards a process that dies before
  // either runs. Failures are logged, never silently dropped.
  process.once("exit", () => {
    if (cached === undefined || disposed) {
      return;
    }
    const resources: StackResources = {
      postgresContainer: cached.stack.postgresContainer,
      serverContainer: cached.stack.serverContainer,
      network: cached.stack.network,
      volumes: cached.stack.volumes,
    };
    for (const failure of disposeStackResourcesSync(resources)) {
      console.error("[ep006] emergency exit cleanup failure:", failure);
    }
  });
  return session;
}
