import { describe, it, expect, vi } from 'vitest';
import { enqueue, replayInOrder, type QueuedMutation } from './offlineQueue';

describe('offlineQueue', () => {
  it('enqueue appends a mutation preserving FIFO order', () => {
    let queue: QueuedMutation[] = [];
    queue = enqueue(queue, 'create_specimen', { accession: 'A' }, 1, 1000);
    queue = enqueue(queue, 'create_subculture', { specimenId: 'A' }, 2, 1001);
    expect(queue.map((m) => m.id)).toEqual([1, 2]);
    expect(queue[0].command).toBe('create_specimen');
    expect(queue[1].command).toBe('create_subculture');
  });

  it('replayInOrder replays every mutation in order when all succeed', async () => {
    const queue: QueuedMutation[] = [
      { id: 1, command: 'create_specimen', args: {}, enqueuedAt: 1 },
      { id: 2, command: 'create_subculture', args: {}, enqueuedAt: 2 },
      { id: 3, command: 'update_specimen', args: {}, enqueuedAt: 3 },
    ];
    const calls: string[] = [];
    const invoke = vi.fn(async (command: string) => {
      calls.push(command);
      return { ok: true };
    });

    const result = await replayInOrder(queue, invoke);

    expect(calls).toEqual(['create_specimen', 'create_subculture', 'update_specimen']);
    expect(result.succeededIds).toEqual([1, 2, 3]);
    expect(result.remaining).toEqual([]);
    expect(result.firstError).toBeNull();
  });

  it('replayInOrder stops at the first failure and never attempts later mutations out of order', async () => {
    const queue: QueuedMutation[] = [
      { id: 1, command: 'create_specimen', args: {}, enqueuedAt: 1 },
      { id: 2, command: 'create_subculture', args: {}, enqueuedAt: 2 },
      { id: 3, command: 'update_specimen', args: {}, enqueuedAt: 3 },
    ];
    const invoke = vi.fn(async (command: string) => {
      if (command === 'create_subculture') throw new Error('specimen not found locally yet');
      return { ok: true };
    });

    const result = await replayInOrder(queue, invoke);

    expect(invoke).toHaveBeenCalledTimes(2); // never reaches update_specimen
    expect(result.succeededIds).toEqual([1]);
    expect(result.remaining.map((m) => m.id)).toEqual([2, 3]);
    expect(result.firstError).toEqual({ id: 2, message: 'specimen not found locally yet' });
  });

  it('replayInOrder on an empty queue resolves immediately without invoking anything', async () => {
    const invoke = vi.fn(async () => ({ ok: true }));
    const result = await replayInOrder([], invoke);
    expect(invoke).not.toHaveBeenCalled();
    expect(result).toEqual({ succeededIds: [], remaining: [], firstError: null });
  });
});
