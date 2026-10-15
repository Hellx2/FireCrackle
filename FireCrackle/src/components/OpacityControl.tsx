import "./OpacityControl.css";

interface Props {
  groupLabel: string;
  toggleLabel: string;
  sliderLabel: string;
  value: number;
  onChange: (value: number) => void;
}

/** A continuous opacity slider with a full/hidden toggle for quick comparisons. */
export function OpacityControl({ groupLabel, toggleLabel, sliderLabel, value, onChange }: Props) {
  return (
    <span className="opacity-control" role="group" aria-label={groupLabel}>
      <button
        type="button"
        aria-label={toggleLabel}
        aria-keyshortcuts="E"
        aria-pressed={value > 0}
        title="Opacity · E · toggle 0% / 100%"
        onClick={() => onChange(value > 0 ? 0 : 1)}
      >
        Opacity
      </button>
      <input
        aria-label={sliderLabel}
        type="range"
        min={0}
        max={1}
        step={0.05}
        value={value}
        onChange={(event) => onChange(event.target.valueAsNumber)}
      />
    </span>
  );
}
