import React from 'react';
import { JsonForms } from '@jsonforms/react';
import { materialCells, materialRenderers } from '@jsonforms/material-renderers';
import schema from '../schemas/researchMaterialSchema';
import uischema from '../schemas/uiSchema';

interface FormComponentProps {
  data: any;
  setData: (data: any) => void;
}

const FormComponent: React.FC<FormComponentProps> = ({ data, setData }) => {
  return (
    <JsonForms
      schema={schema}
      uischema={uischema}
      data={data}
      renderers={materialRenderers}
      cells={materialCells}
      onChange={({ data: newData }) => setData(newData)}
    />
  );
};

export default FormComponent;