# Load .NET libraries
Add-Type -TypeDefinition @"
using System;
using System.IO;
using System.IO.Pipes;
using System.Text;

public class PipeClient {
    public static void Main() {
        using (var pipe = new NamedPipeClientStream(".", "as_ipc_poc_pipe", PipeDirection.InOut)) {
            pipe.Connect();
            Console.WriteLine("Connected to pipe.");

            using (var writer = new StreamWriter(pipe, Encoding.UTF8, 1024, true))
            using (var reader = new BinaryReader(pipe)) {
                writer.AutoFlush = true;
                
                // Send request
                string request = "321:./images/12 jpegs 2/1.jpg";
                writer.WriteLine(request);
                Console.WriteLine("Sent request: " + request);
                
                // Read the length prefix (first 4 bytes)
                byte[] lengthBytes = reader.ReadBytes(4);
                if (lengthBytes.Length < 4) {
                    Console.WriteLine("Failed to read length prefix.");
                    return;
                }
                int messageLength = BitConverter.ToInt32(lengthBytes, 0);
                Console.WriteLine("Expected message length: " + messageLength + " bytes");

                // Read the actual message
                byte[] responseBytes = reader.ReadBytes(messageLength);
                if (responseBytes.Length < messageLength) {
                    Console.WriteLine("Incomplete response received.");
                    return;
                }
                
                // Convert response to Base64 for logging
                string base64Response = Convert.ToBase64String(responseBytes);
                Console.WriteLine("Received binary response (Base64 encoded):");
                Console.WriteLine(base64Response);
                
                // (Optional) Save response to file
                File.WriteAllBytes("response.bin", responseBytes);
                Console.WriteLine("Response saved to 'response.bin'.");
            }
        }
    }
}
"@ -Language CSharp -ReferencedAssemblies "System.IO.Pipes"

# Run the C# PipeClient
[PipeClient]::Main()
