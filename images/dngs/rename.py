import os

# Path to the directory containing your jpg images
directory = r"C:\\Users\danie\\projects\\ipc-poc-backend\\images\\dngs"

# Get all the .jpg files in the directory
files = [f for f in os.listdir(directory) if f.lower().endswith('.dng')]

# Sort files alphabetically (optional, in case the files are not in the order you want)
files.sort()

# Loop through the files and rename them sequentially
for index, file in enumerate(files, start=1):
    # Get the full path of the current file
    old_path = os.path.join(directory, file)
    
    # Create the new file name (e.g., 1.jpg, 2.jpg, etc.)
    new_file_name = f"{index}.jpg"
    
    # Get the full path for the new file name
    new_path = os.path.join(directory, new_file_name)
    
    # Rename the file
    os.rename(old_path, new_path)

    print(f"Renamed {file} to {new_file_name}")

print("Renaming completed.")
