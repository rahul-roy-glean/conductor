import { describe, it, expect, vi, beforeEach } from 'vitest';
import { render, screen } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import NudgeDialog from './NudgeDialog';

// Mock the API client
const mockNudgeAgent = vi.fn(() => Promise.resolve());
vi.mock('../api/client', () => ({
  nudgeAgent: (...args: unknown[]) => mockNudgeAgent(...args),
}));

beforeEach(() => {
  mockNudgeAgent.mockClear();
  mockNudgeAgent.mockImplementation(() => Promise.resolve());
});

describe('NudgeDialog', () => {
  it('renders the dialog with title, textarea, and buttons', () => {
    const onClose = vi.fn();
    render(<NudgeDialog agentId="agent-1" onClose={onClose} />);

    expect(screen.getByText('Nudge Agent')).toBeInTheDocument();
    expect(screen.getByPlaceholderText('Enter nudge message...')).toBeInTheDocument();
    expect(screen.getByText('Cancel')).toBeInTheDocument();
    expect(screen.getByText('Send')).toBeInTheDocument();
  });

  it('textarea accepts input', async () => {
    const user = userEvent.setup();
    const onClose = vi.fn();
    render(<NudgeDialog agentId="agent-1" onClose={onClose} />);

    const textarea = screen.getByPlaceholderText('Enter nudge message...');
    await user.type(textarea, 'Hello agent');

    expect(textarea).toHaveValue('Hello agent');
  });

  it('send button is disabled when message is empty', () => {
    const onClose = vi.fn();
    render(<NudgeDialog agentId="agent-1" onClose={onClose} />);

    const sendButton = screen.getByText('Send');
    expect(sendButton).toBeDisabled();
  });

  it('send button is disabled when message is only whitespace', async () => {
    const user = userEvent.setup();
    const onClose = vi.fn();
    render(<NudgeDialog agentId="agent-1" onClose={onClose} />);

    const textarea = screen.getByPlaceholderText('Enter nudge message...');
    await user.type(textarea, '   ');

    const sendButton = screen.getByText('Send');
    expect(sendButton).toBeDisabled();
  });

  it('send button calls nudgeAgent with agent ID and message, then calls onClose', async () => {
    const user = userEvent.setup();
    const onClose = vi.fn();
    render(<NudgeDialog agentId="agent-42" onClose={onClose} />);

    const textarea = screen.getByPlaceholderText('Enter nudge message...');
    await user.type(textarea, 'Please continue');

    const sendButton = screen.getByText('Send');
    await user.click(sendButton);

    expect(mockNudgeAgent).toHaveBeenCalledWith('agent-42', 'Please continue');
    // After successful send, onClose should be called
    await vi.waitFor(() => {
      expect(onClose).toHaveBeenCalled();
    });
  });

  it('cancel button calls onClose', async () => {
    const user = userEvent.setup();
    const onClose = vi.fn();
    render(<NudgeDialog agentId="agent-1" onClose={onClose} />);

    const cancelButton = screen.getByText('Cancel');
    await user.click(cancelButton);

    expect(onClose).toHaveBeenCalledTimes(1);
  });

  it('clicking the backdrop overlay calls onClose', async () => {
    const user = userEvent.setup();
    const onClose = vi.fn();
    const { container } = render(<NudgeDialog agentId="agent-1" onClose={onClose} />);

    // The backdrop is the outermost fixed div
    const backdrop = container.querySelector('.fixed.inset-0');
    expect(backdrop).toBeInTheDocument();
    await user.click(backdrop!);

    expect(onClose).toHaveBeenCalled();
  });

  it('clicking inside the dialog does not call onClose', async () => {
    const user = userEvent.setup();
    const onClose = vi.fn();
    render(<NudgeDialog agentId="agent-1" onClose={onClose} />);

    const title = screen.getByText('Nudge Agent');
    await user.click(title);

    expect(onClose).not.toHaveBeenCalled();
  });

  it('shows Sending... text while sending', async () => {
    const user = userEvent.setup();
    const onClose = vi.fn();
    // Make nudgeAgent hang so we can check the sending state
    let resolveNudge: () => void;
    mockNudgeAgent.mockImplementation(() => new Promise<void>((resolve) => { resolveNudge = resolve; }));

    render(<NudgeDialog agentId="agent-1" onClose={onClose} />);

    const textarea = screen.getByPlaceholderText('Enter nudge message...');
    await user.type(textarea, 'test message');
    await user.click(screen.getByText('Send'));

    expect(screen.getByText('Sending...')).toBeInTheDocument();

    // Resolve the promise to clean up
    resolveNudge!();
    await vi.waitFor(() => {
      expect(onClose).toHaveBeenCalled();
    });
  });
});
