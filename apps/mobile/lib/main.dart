import 'package:flutter/material.dart';

/// Nexus mobile entry point (EP-034). The contract layer is
/// framework-neutral; this shell binds it to Flutter.
void main() {
  runApp(const NexusMobileApp());
}

class NexusMobileApp extends StatelessWidget {
  const NexusMobileApp({super.key});

  @override
  Widget build(BuildContext context) {
    return MaterialApp(
      title: 'Nexus',
      theme: ThemeData(colorSchemeSeed: Colors.indigo),
      home: const NexusHome(),
    );
  }
}

class NexusHome extends StatelessWidget {
  const NexusHome({super.key});

  @override
  Widget build(BuildContext context) {
    return Scaffold(
      appBar: AppBar(title: const Text('Nexus')),
      body: const Center(child: Text('Nexus mobile contract layer ready.')),
    );
  }
}
