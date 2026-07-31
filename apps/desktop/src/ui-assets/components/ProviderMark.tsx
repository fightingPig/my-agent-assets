import claudeCodeMark from "@lobehub/icons-static-svg/icons/claude-color.svg";
import codexMark from "@lobehub/icons-static-svg/icons/codex.svg";
import type { SupportedMountProvider } from "../contracts";

export type { SupportedMountProvider } from "../contracts";

type ProviderMarkProps = {
  provider: SupportedMountProvider;
  size?: number;
  withLabel?: boolean;
};

const providerMetadata = {
  claude_code: {
    label: "Claude Code",
    src: claudeCodeMark,
  },
  codex: {
    label: "Codex",
    src: codexMark,
  },
} as const;

export function ProviderMark({ provider, size = 18, withLabel = false }: ProviderMarkProps) {
  const metadata = providerMetadata[provider];
  return (
    <span className={`maa-provider-mark provider-${provider}`}>
      <img
        alt=""
        aria-hidden="true"
        height={size}
        src={metadata.src}
        width={size}
      />
      {withLabel ? <span>{metadata.label}</span> : null}
    </span>
  );
}

export function providerLabel(provider: SupportedMountProvider) {
  return providerMetadata[provider].label;
}
