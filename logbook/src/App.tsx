import React, { useState } from 'react';
import FormComponent from './components/FormComponent';
import FileUploadComponent from './components/FileUploadComponent';

const App: React.FC = () => {
  const [formData, setFormData] = useState<any>({});

  const handleFileUpload = (json: any): void => {
    setFormData(json);
  };

  const handleExport = (): void => {
    const dataStr = JSON.stringify(formData, null, 2);
    const blob = new Blob([dataStr], { type: 'application/json' });
    const url = URL.createObjectURL(blob);
    const link = document.createElement('a');
    link.href = url;
    link.download = 'research_material_data.json';
    link.click();
    URL.revokeObjectURL(url);
  };

  return (
    <div style={{ margin: '20px' }}>
      <h1>Research Material Logbook Form</h1>
      <div style={{ marginBottom: '20px' }}>
        <FileUploadComponent onFileUpload={handleFileUpload} />
      </div>
      <FormComponent data={formData} setData={setFormData} />
      <div style={{ marginTop: '20px' }}>
        <button onClick={handleExport}>Export as JSON</button>
      </div>
    </div>
  );
};

export default App;