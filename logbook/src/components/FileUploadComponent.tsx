import React from 'react';

interface FileUploadComponentProps {
  onFileUpload: (json: any) => void;
}

const FileUploadComponent: React.FC<FileUploadComponentProps> = ({ onFileUpload }) => {
  const handleFileUpload = (event: React.ChangeEvent<HTMLInputElement>) => {
    const file = event.target.files?.[0];
    if (file) {
      const reader = new FileReader();
      reader.onload = (e) => {
        if (e.target?.result) {
          const json = JSON.parse(e.target.result as string);
          onFileUpload(json);
        }
      };
      reader.readAsText(file);
    }
  };

  return <input type="file" accept=".json" onChange={handleFileUpload} />;
};

export default FileUploadComponent;