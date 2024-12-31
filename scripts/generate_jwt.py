#!/usr/bin/env python3

import jwt
import os
import time
import argparse
from datetime import datetime

def generate_token(secret=None):
    # Get JWT secret from argument or environment variable
    jwt_secret = secret or os.getenv('JWT_SECRET')
    if not jwt_secret:
        print("Error: No JWT secret provided")
        print("Please either:")
        print("  1. Set environment variable: export JWT_SECRET=your_secret_here")
        print("  2. Provide secret as argument: ./generate_jwt.py --secret your_secret_here")
        return

    # Set expiration to 24 hours from now
    exp = int(time.time()) + 86400  # Current time + 24 hours

    # Create claims
    claims = {
        'exp': exp
    }

    try:
        # Generate token
        token = jwt.encode(claims, jwt_secret, algorithm='HS256')
        
        # Print results
        print("\nGenerated JWT Token. To use in Swagger UI, copy this token and then enter it in the authorize field of the UI:")
        print(f"{token}\n")
        print("To use in API requests, add the Bearer prefix:")
        print(f"Bearer {token}\n")
        
        print("Token will expire at:", 
              datetime.fromtimestamp(exp).strftime('%Y-%m-%d %H:%M:%S'), "(in 24 hours)\n")
        
    except Exception as e:
        print(f"Error generating token: {e}")

if __name__ == "__main__":
    parser = argparse.ArgumentParser(description='Generate JWT token')
    parser.add_argument('--secret', type=str, help='JWT secret key')
    args = parser.parse_args()
    
    generate_token(args.secret)
