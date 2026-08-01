import { describe, expect, it } from "vitest";
import {
  DEFAULT_ASSET_CENTER_PATH,
  DEFAULT_ASSET_REPOSITORY_SLUG,
  DEFAULT_GITHUB_HTTPS_REMOTE_TEMPLATE,
  DEFAULT_GITHUB_SSH_REMOTE_TEMPLATE,
  DEFAULT_GIT_REMOTE_NAME,
} from "./defaults";

describe("desktop defaults", () => {
  it("keeps the asset center and remote repository naming contract stable", () => {
    expect(DEFAULT_ASSET_CENTER_PATH).toBe("~/.my-agent-assets-data");
    expect(DEFAULT_ASSET_REPOSITORY_SLUG).toBe("my-agent-assets-data");
    expect(DEFAULT_GIT_REMOTE_NAME).toBe("origin");
    expect(DEFAULT_GITHUB_SSH_REMOTE_TEMPLATE).toBe(
      "git@github.com:<owner>/my-agent-assets-data.git",
    );
    expect(DEFAULT_GITHUB_HTTPS_REMOTE_TEMPLATE).toBe(
      "https://github.com/<owner>/my-agent-assets-data.git",
    );
  });
});
