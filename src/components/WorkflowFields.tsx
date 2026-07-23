import type { Field, ToolInput } from "../lib/registry";
import { FieldControl } from "./FieldControl";

interface WorkflowFieldsProps {
  fields: Field[];
  values: ToolInput;
  onChange: (key: string, value: unknown) => void;
}

export function WorkflowFields({
  fields,
  values,
  onChange,
}: WorkflowFieldsProps) {
  return (
    <>
      {fields.map((field) => (
        <div className="field" key={field.key}>
          <div className="field-label">
            <label>{field.label}</label>
            {field.help && <span>{field.help}</span>}
          </div>
          <FieldControl
            field={field}
            value={values[field.key]}
            onChange={(value) => onChange(field.key, value)}
          />
        </div>
      ))}
    </>
  );
}
